//! Native's window services on Windows, through the DWM and the window's system menu.

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

use core::ffi::c_void;
use core::pin::Pin;

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW,
    DWMWA_SYSTEMBACKDROP_TYPE,
};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    EnableMenuItem, GetSystemMenu, IsZoomed, PostMessageW, SetMenuDefaultItem, TrackPopupMenu,
    MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, SC_CLOSE, SC_MAXIMIZE, SC_MINIMIZE, SC_MOVE, SC_RESTORE,
    SC_SIZE, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_SYSCOMMAND,
};

use ffi::QWindow;

fn hwnd(window: &QWindow) -> HWND {
    HWND(usize::from(window.win_id()) as *mut c_void)
}

pub fn set_blur_behind(window: Pin<&mut QWindow>, enabled: bool) -> bool {
    let hwnd = hwnd(&window);
    let backdrop = if enabled {
        DWMSBT_TRANSIENTWINDOW
    } else {
        DWMSBT_NONE
    };
    // the backdrop shows through the client area where the frame is extended into it
    let extent = if enabled { -1 } else { 0 };
    let margins = MARGINS {
        cxLeftWidth: extent,
        cxRightWidth: extent,
        cyTopHeight: extent,
        cyBottomHeight: extent,
    };
    // SAFETY: the window is the QWindow's, and the attribute's value is a DWM_SYSTEMBACKDROP_TYPE
    unsafe {
        _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&raw const backdrop).cast(),
            size_of_val(&backdrop) as u32,
        )
        .is_ok()
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
