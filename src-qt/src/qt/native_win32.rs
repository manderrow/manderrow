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
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_MAINWINDOW, DWMSBT_NONE,
    DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWINDOWATTRIBUTE,
};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    EnableMenuItem, GetSystemMenu, GetWindowLongPtrW, GetWindowRect, IsZoomed, PostMessageW,
    SetMenuDefaultItem, SetWindowLongPtrW, SetWindowPos, TrackPopupMenu, GWL_STYLE, HTCLIENT,
    HTTOP, HTTOPLEFT, HTTOPRIGHT, MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, NCCALCSIZE_PARAMS, SC_CLOSE,
    SC_MAXIMIZE, SC_MINIMIZE, SC_MOVE, SC_RESTORE, SC_SIZE, SM_CXPADDEDBORDER, SM_CYSIZEFRAME,
    STYLESTRUCT, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
    SWP_NOZORDER, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST,
    WM_STYLECHANGING, WM_SYSCOMMAND, WS_SYSMENU, WS_THICKFRAME,
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
    // redirection surface, which nothing draws in. With a backdrop, DWM shows that surface as
    // opaque black, and white where the window was first shown, rather than the backdrop. Only
    // Direct3D can draw into windows made without the surface, which Qt Quick uses unless it is
    // told otherwise.
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

/// How the window's background is blurred on Windows, which MANDERROW_DEV_WINDOWS_BLUR picks
/// while the methods are being compared: "acrylic" (the default), "mica", or "blur".
#[derive(Clone, Copy, PartialEq, Eq)]
enum BlurMethod {
    /// The acrylic backdrop, tinted by Windows' light or dark theme
    Acrylic,
    /// The Mica backdrop, which is tinted by the desktop's wallpaper, not what is behind the window
    Mica,
    /// A plain blur, without a tint, through the undocumented accent policy of Windows 10 and 11
    Blur,
}

fn blur_method() -> BlurMethod {
    match std::env::var("MANDERROW_DEV_WINDOWS_BLUR").as_deref() {
        Ok("mica") => BlurMethod::Mica,
        Ok("blur") => BlurMethod::Blur,
        _ => BlurMethod::Acrylic,
    }
}

pub fn set_blur_behind(window: Pin<&mut QWindow>, enabled: bool) -> bool {
    let hwnd = hwnd(&window);
    let method = blur_method();
    // The backdrops are drawn behind the window, and show through the client area where the
    // frame is extended into it. Elsewhere, and before Windows 11, which has no backdrops, the
    // client area is transparent rather than blurred, as Qt makes windows with an alpha channel.
    // The frame is extended from the left side alone, by more than any window's width: extended
    // from the top, it brings back the title bar's accent color, when Windows is set to show it.
    let backdrop = match (enabled, method) {
        (true, BlurMethod::Acrylic) => DWMSBT_TRANSIENTWINDOW,
        (true, BlurMethod::Mica) => DWMSBT_MAINWINDOW,
        _ => DWMSBT_NONE,
    };
    // SAFETY: the window is the QWindow's, and the value is a DWM_SYSTEMBACKDROP_TYPE
    let backdrop_set = unsafe { set_attribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, backdrop) };
    // SAFETY: the window is the QWindow's
    let accent_set = unsafe { set_accent_blur(hwnd, enabled && method == BlurMethod::Blur) };
    let extended = enabled && method != BlurMethod::Blur && backdrop_set;
    let margins = MARGINS {
        cxLeftWidth: if extended { 0x10000 } else { 0 },
        ..MARGINS::default()
    };
    // SAFETY: the window is the QWindow's
    _ = unsafe { DwmExtendFrameIntoClientArea(hwnd, &margins) };
    match method {
        BlurMethod::Blur => enabled == accent_set,
        _ => enabled == backdrop_set,
    }
}

/// Blurs what is behind the window, or stops, with the accent policy that Windows' own shell
/// uses, through a function of user32 that is not in its import library. Returns whether it could.
///
/// # Safety
///
/// The window must be valid.
unsafe fn set_accent_blur(hwnd: HWND, enabled: bool) -> bool {
    #[repr(C)]
    struct AccentPolicy {
        state: u32,
        flags: u32,
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
    const ACCENT_DISABLED: u32 = 0;
    const ACCENT_ENABLE_BLURBEHIND: u32 = 3;

    // SAFETY: user32 is loaded for as long as the app has windows, and the function has this
    // signature in Windows 10 and 11
    unsafe {
        let Ok(user32) = GetModuleHandleW(w!("user32.dll")) else {
            return false;
        };
        let Some(function) = GetProcAddress(user32, s!("SetWindowCompositionAttribute")) else {
            return false;
        };
        let function: SetWindowCompositionAttribute = core::mem::transmute(function);
        let mut policy = AccentPolicy {
            state: if enabled {
                ACCENT_ENABLE_BLURBEHIND
            } else {
                ACCENT_DISABLED
            },
            flags: 0,
            gradient_color: 0,
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
        // Windows draws the title bar's buttons where the frame is extended into the client area,
        // where they show through the app's own when the window is translucent, unless the window
        // has no system menu. The menu, once made, is kept for the app to show, though.
        _ = GetSystemMenu(hwnd, false);
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(WS_SYSMENU.0 as isize));
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

/// The undocumented message with which the taskbar asks a window to show its system menu, at a
/// point on the screen. The window answers it itself, as Windows may not without WS_SYSMENU.
const WM_POPUPSYSTEMMENU: u32 = 0x313;

/// The point on the screen of a mouse message, whose coordinates are signed 16-bit numbers.
fn screen_point(lparam: LPARAM) -> POINT {
    POINT {
        x: i32::from(lparam.0 as u16 as i16),
        y: i32::from((lparam.0 >> 16) as u16 as i16),
    }
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
                let POINT { x, y } = screen_point(lparam);
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
            // Qt and others change the style when the window enters and leaves full screen
            WM_STYLECHANGING if wparam.0 as i32 == GWL_STYLE.0 => {
                let styles = &mut *(lparam.0 as *mut STYLESTRUCT);
                styles.styleNew &= !WS_SYSMENU.0;
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_POPUPSYSTEMMENU => {
                track_window_menu(hwnd, screen_point(lparam));
                LRESULT(0)
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
    let mut point = POINT {
        x: (x * scale) as i32,
        y: (y * scale) as i32,
    };
    // SAFETY: the window is the QWindow's
    unsafe {
        _ = ClientToScreen(hwnd, &mut point);
        track_window_menu(hwnd, point)
    }
}

/// Shows the window's system menu at a point on the screen, and carries out the command chosen
/// from it, if any. Returns whether the window has the menu.
///
/// # Safety
///
/// The window must be valid. It outlives the menu, which is modal.
unsafe fn track_window_menu(hwnd: HWND, point: POINT) -> bool {
    // SAFETY: the window is valid
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
