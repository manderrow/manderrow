//! Native's window services on Windows, through the DWM, the window's system menu, and a subclass
//! of the window that takes over its non-client area.

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qtypes.h");
        type quintptr = cxx_qt_lib::quintptr;

        include!(<QtGui/QWindow>);
        type QWindow = crate::qt::native::qobject::QWindow;

        #[rust_name = "win_id"]
        fn winId(self: &QWindow) -> quintptr;
        #[rust_name = "device_pixel_ratio"]
        fn devicePixelRatio(self: &QWindow) -> f64;
    }
}

use core::ffi::{c_char, c_int, c_void};
use core::pin::Pin;

use windows::core::{s, w, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWINDOWATTRIBUTE, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::{ClientToScreen, CreateRectRgn, DeleteObject};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    EnableMenuItem, GetSystemMenu, GetWindowLongPtrW, GetWindowRect, IsZoomed, PostMessageW,
    SetMenuDefaultItem, SetWindowPos, TrackPopupMenu, GWL_STYLE, HTCLIENT, HTTOP, HTTOPLEFT,
    HTTOPRIGHT, MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, NCCALCSIZE_PARAMS, SC_CLOSE, SC_MAXIMIZE,
    SC_MINIMIZE, SC_MOVE, SC_RESTORE, SC_SIZE, SM_CXPADDEDBORDER, SM_CYSIZEFRAME, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST, WM_SYSCOMMAND, WS_THICKFRAME,
};

use ffi::QWindow;

// The C runtime's copy of the environment, which is the one Qt reads, and which
// `std::env::set_var` doesn't change
unsafe extern "C" {
    fn _putenv_s(name: *const c_char, value: *const c_char) -> c_int;
}

/// Configuration that must happen before the `QGuiApplication` is created.
pub fn configure_before_app() {
    // Qt Quick draws with Direct3D into a DirectComposition visual, over the window's GDI
    // redirection surface, which nothing draws in, and which can come between the visual and a
    // blur behind the window, as a white box where the window was first shown. Windows made
    // without the surface, as Zed's GPUI makes them, have only the visual. Only Direct3D can
    // draw into them, which Qt Quick uses unless it is told otherwise.
    let var = |name| std::env::var_os(name).filter(|value| !value.is_empty());
    let direct3d = var("QSG_RHI_BACKEND").is_none_or(|backend| {
        !matches!(
            backend.to_str(),
            Some("gl" | "gles2" | "opengl" | "vulkan" | "null")
        )
    });
    let rhi = var("QT_QUICK_BACKEND").is_none_or(|backend| backend == "rhi");
    if direct3d && rhi && var("QT_QPA_DISABLE_REDIRECTION_SURFACE").is_none() {
        // SAFETY: the strings are NUL-terminated, and the C runtime locks its environment
        unsafe {
            _putenv_s(
                c"QT_QPA_DISABLE_REDIRECTION_SURFACE".as_ptr(),
                c"1".as_ptr(),
            )
        };
    }
}

fn hwnd(window: &QWindow) -> HWND {
    HWND(usize::from(window.win_id()) as *mut c_void)
}

/// Sets a DWM attribute of the window to a value of the type it takes.
///
/// # Safety
///
/// `T` must be the type of the attribute's value.
unsafe fn set_attribute<T>(hwnd: HWND, attribute: DWMWINDOWATTRIBUTE, value: T) -> bool {
    // SAFETY: the caller promises that the value is the attribute's
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            attribute,
            (&raw const value).cast(),
            size_of::<T>() as u32,
        )
        .is_ok()
    }
}

pub fn set_blur_behind(window: Pin<&mut QWindow>, enabled: bool) -> bool {
    let hwnd = hwnd(&window);
    // The window is blurred with the acrylic accent of Windows' own shell, as Zed's GPUI, which
    // draws the same way, blurs its windows. Unlike Windows 11's documented backdrops, its tint
    // can be all but transparent, leaving the app's own, as on Linux, and it needs neither
    // Windows 11 22H2 nor the frame to be extended into the client area, where Windows would draw
    // its title bar buttons. With transparency effects off, or energy saver on, Windows draws it
    // black. It replaces the blur behind the window that Qt asks for, which only lets the
    // window's transparent parts show through, and which is asked for again once it is gone.
    // SAFETY: the window is the QWindow's
    unsafe {
        if enabled {
            _ = set_blur_behind_region(hwnd, false);
            set_accent(hwnd, ACCENT_ENABLE_ACRYLICBLURBEHIND)
        } else {
            set_accent(hwnd, ACCENT_DISABLED) && set_blur_behind_region(hwnd, true)
        }
    }
}

const ACCENT_DISABLED: u32 = 0;
const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;

/// Sets the window's accent, through a function of user32 that is undocumented, and not in its
/// import library, but has been there since Windows 10. Returns whether it could.
///
/// # Safety
///
/// The window must be valid.
unsafe fn set_accent(hwnd: HWND, state: u32) -> bool {
    #[repr(C)]
    struct AccentPolicy {
        state: u32,
        flags: u32,
        /// The tint, as 0xAABBGGRR, which mustn't be fully transparent
        gradient_color: u32,
        animation_id: u32,
    }
    #[repr(C)]
    struct CompositionAttributeData {
        attribute: u32,
        data: *mut c_void,
        size: usize,
    }
    type SetWindowCompositionAttribute =
        unsafe extern "system" fn(HWND, *mut CompositionAttributeData) -> BOOL;
    const WCA_ACCENT_POLICY: u32 = 19;

    // SAFETY: user32 is loaded for as long as the app has windows, and the function has this
    // signature since Windows 10
    unsafe {
        let Ok(user32) = GetModuleHandleW(w!("user32.dll")) else {
            return false;
        };
        let Some(function) = GetProcAddress(user32, s!("SetWindowCompositionAttribute")) else {
            return false;
        };
        let function: SetWindowCompositionAttribute = core::mem::transmute(function);
        let mut policy = AccentPolicy {
            state,
            flags: 0,
            gradient_color: 0x0100_0000,
            animation_id: 0,
        };
        let mut data = CompositionAttributeData {
            attribute: WCA_ACCENT_POLICY,
            data: (&raw mut policy).cast(),
            size: size_of::<AccentPolicy>(),
        };
        function(hwnd, &mut data).as_bool()
    }
}

/// Turns on or off the blur behind the window with an empty region, which, as Qt asks for it, only
/// lets the window's transparent parts show what is behind it.
///
/// # Safety
///
/// The window must be valid.
unsafe fn set_blur_behind_region(hwnd: HWND, enabled: bool) -> bool {
    // SAFETY: the window is valid, and the region is deleted once DWM has copied it
    unsafe {
        let region = if enabled {
            CreateRectRgn(0, 0, -1, -1)
        } else {
            Default::default()
        };
        let blur = DWM_BLURBEHIND {
            dwFlags: if enabled {
                DWM_BB_ENABLE | DWM_BB_BLURREGION
            } else {
                DWM_BB_ENABLE
            },
            fEnable: enabled.into(),
            hRgnBlur: region,
            fTransitionOnMaximized: false.into(),
        };
        let ok = DwmEnableBlurBehindWindow(hwnd, &blur).is_ok();
        if !region.is_invalid() {
            _ = DeleteObject(region.into());
        }
        ok
    }
}

pub fn set_dark_frame(window: Pin<&mut QWindow>, dark: bool) -> bool {
    // SAFETY: the window is the QWindow's, and the value is a BOOL
    unsafe {
        set_attribute(
            hwnd(&window),
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            BOOL::from(dark),
        )
    }
}

/// The id of the subclass that hides a window's title bar, leaving the rest of its frame, so that
/// it keeps its resize borders, shadow, rounded corners, snapping, and animations, which Windows
/// only gives windows with a frame.
const FRAME_SUBCLASS: usize = 1;

pub fn hide_title_bar(window: Pin<&mut QWindow>) -> bool {
    let hwnd = hwnd(&window);
    // SAFETY: the window is the QWindow's, and the subclass removes itself when it is destroyed
    unsafe {
        if !SetWindowSubclass(hwnd, Some(frame_subclass), FRAME_SUBCLASS, 0).as_bool() {
            return false;
        }
        // recalculates the client area with the subclass
        _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED
                | SWP_NOMOVE
                | SWP_NOSIZE
                | SWP_NOZORDER
                | SWP_NOOWNERZORDER
                | SWP_NOACTIVATE,
        );
    }
    true
}

/// The thickness of the window's resize borders, as Windows sizes them.
///
/// # Safety
///
/// The window must be valid.
unsafe fn resize_border(hwnd: HWND) -> i32 {
    // SAFETY: the window is valid
    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
    }
}

unsafe extern "system" fn frame_subclass(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    // SAFETY: Windows passes the subclassed window and its message's parameters
    unsafe {
        // Qt takes the frame away in full screen, where Windows' own handling is right
        let framed = (GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_THICKFRAME.0) != 0;
        match message {
            // The client area is the window without its side and bottom borders, as Windows would
            // make it without the title bar. A maximized window's borders lie beyond the screen,
            // as do its top ones. Qt doesn't see this, so that it doesn't take the margins of the
            // frame from Windows' own calculation, and finds them from the window's rectangles
            // instead.
            WM_NCCALCSIZE if wparam.0 != 0 && framed => {
                let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
                let client = &mut params.rgrc[0];
                let border = resize_border(hwnd);
                client.left += border;
                client.right -= border;
                client.bottom -= border;
                if IsZoomed(hwnd).as_bool() {
                    client.top += border;
                }
                LRESULT(0)
            }
            // The top border is within the client area, so it is found there.
            WM_NCHITTEST if framed => {
                let hit = DefSubclassProc(hwnd, message, wparam, lparam);
                if hit.0 != HTCLIENT as isize || IsZoomed(hwnd).as_bool() {
                    return hit;
                }
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_err() {
                    return hit;
                }
                // the screen coordinates of the point, which are signed 16-bit numbers
                let x = i32::from(lparam.0 as u16 as i16);
                let y = i32::from((lparam.0 >> 16) as u16 as i16);
                let border = resize_border(hwnd);
                if y >= rect.top + border {
                    hit
                } else if x < rect.left + 2 * border {
                    LRESULT(HTTOPLEFT as isize)
                } else if x >= rect.right - 2 * border {
                    LRESULT(HTTOPRIGHT as isize)
                } else {
                    LRESULT(HTTOP as isize)
                }
            }
            WM_NCDESTROY => {
                _ = RemoveWindowSubclass(hwnd, Some(frame_subclass), FRAME_SUBCLASS);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }
}

pub fn show_window_menu(window: Pin<&mut QWindow>, x: f64, y: f64) -> bool {
    let hwnd = hwnd(&window);
    let scale = window.device_pixel_ratio();
    // SAFETY: the window is the QWindow's, which outlives the menu, which is modal
    unsafe {
        let menu = GetSystemMenu(hwnd, false);
        if menu.is_invalid() {
            return false;
        }
        // the menu's items follow the window's state, as they do for a native title bar
        let maximized = IsZoomed(hwnd).as_bool();
        let enable = |command, enabled: bool| {
            _ = EnableMenuItem(
                menu,
                command,
                MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_GRAYED },
            );
        };
        enable(SC_RESTORE, maximized);
        enable(SC_MOVE, !maximized);
        enable(SC_SIZE, !maximized);
        enable(SC_MAXIMIZE, !maximized);
        enable(SC_MINIMIZE, true);
        enable(SC_CLOSE, true);
        _ = SetMenuDefaultItem(menu, SC_CLOSE, 0);
        let mut point = POINT {
            x: (x * scale) as i32,
            y: (y * scale) as i32,
        };
        _ = ClientToScreen(hwnd, &mut point);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        );
        if command.0 != 0 {
            _ = PostMessageW(
                Some(hwnd),
                WM_SYSCOMMAND,
                WPARAM(command.0 as usize),
                LPARAM(0),
            );
        }
    }
    true
}

pub fn tiled_edges(_window: Pin<&mut QWindow>) -> i32 {
    // Snap Layouts don't tell windows that they are snapped
    0
}

pub fn place_window_buttons(_window: Pin<&mut QWindow>, _bar_height: f64) -> f64 {
    // the app draws its own buttons
    0.0
}
