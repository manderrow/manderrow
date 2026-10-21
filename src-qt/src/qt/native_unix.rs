//! Native's window services on Linux and the BSDs, through KDE's and Wayland's libraries. They are
//! loaded rather than linked, so that the app doesn't depend on them.

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("manderrow/cpp/wayland.h");
        type QWindow = crate::qt::native::qobject::QWindow;

        type wl_proxy;
        type wl_seat;
        #[namespace = "QNativeInterface"]
        type QWaylandApplication;

        #[rust_name = "last_input_seat"]
        fn lastInputSeat(self: &QWaylandApplication) -> *mut wl_seat;
        #[rust_name = "last_input_serial"]
        fn lastInputSerial(self: &QWaylandApplication) -> u32;
    }

    #[namespace = "manderrow"]
    unsafe extern "C++" {
        #[rust_name = "wayland_application"]
        fn waylandApplication() -> *mut QWaylandApplication;
        #[rust_name = "xdg_toplevel"]
        fn xdgToplevel(window: Pin<&mut QWindow>) -> *mut wl_proxy;
    }
}

use core::ffi::{c_int, c_void, CStr};
use core::pin::Pin;
use std::sync::LazyLock;

use cxx_qt_lib::QRegion;

use ffi::{wl_proxy, QWindow};

/// A shared library, which stays loaded once it is.
struct Library(*mut c_void);

impl Library {
    fn open(name: &CStr, flags: c_int) -> Option<Self> {
        // SAFETY: the name is a C string
        let handle = unsafe { libc::dlopen(name.as_ptr(), flags) };
        (!handle.is_null()).then_some(Self(handle))
    }

    /// # Safety
    ///
    /// `F` must be the type of a pointer to the function, if there is one.
    unsafe fn function<F: Copy>(&self, name: &CStr) -> Option<F> {
        // SAFETY: the handle is the library's, and the name is a C string
        let symbol = unsafe { libc::dlsym(self.0, name.as_ptr()) };
        // SAFETY: the caller promises that F is the function's type
        (!symbol.is_null()).then(|| unsafe { core::mem::transmute_copy(&symbol) })
    }
}

/// `KWindowEffects::enableBlurBehind(QWindow*, bool, const QRegion&)`, which is called as C's
/// functions are, as its arguments are all pointers and integers.
type EnableBlurBehind = unsafe extern "C" fn(*mut QWindow, bool, *const QRegion);

static ENABLE_BLUR_BEHIND: LazyLock<Option<EnableBlurBehind>> = LazyLock::new(|| {
    let library = Library::open(
        c"libKF6WindowSystem.so.6",
        libc::RTLD_NOW | libc::RTLD_LOCAL,
    )?;
    // SAFETY: the symbol is the function
    unsafe { library.function(c"_ZN14KWindowEffects16enableBlurBehindEP7QWindowbRK7QRegion") }
});

/// The functions of libwayland-client that Qt has loaded.
struct Wayland {
    marshal_flags:
        unsafe extern "C" fn(*mut wl_proxy, u32, *const c_void, u32, u32, ...) -> *mut wl_proxy,
    get_version: unsafe extern "C" fn(*mut wl_proxy) -> u32,
}

static WAYLAND: LazyLock<Option<Wayland>> = LazyLock::new(|| {
    let library = Library::open(
        c"libwayland-client.so.0",
        libc::RTLD_NOW | libc::RTLD_NOLOAD,
    )?;
    // SAFETY: the symbols are the functions
    unsafe {
        Some(Wayland {
            marshal_flags: library.function(c"wl_proxy_marshal_flags")?,
            get_version: library.function(c"wl_proxy_get_version")?,
        })
    }
});

pub fn set_blur_behind(window: Pin<&mut QWindow>, enabled: bool) -> bool {
    let Some(enable_blur_behind) = *ENABLE_BLUR_BEHIND else {
        return false;
    };
    // an empty region is the whole window
    let region = QRegion::default();
    // SAFETY: the function only uses the window and region for the call
    unsafe { enable_blur_behind(window.get_unchecked_mut(), enabled, &region) };
    true
}

pub fn set_dark_frame(_window: Pin<&mut QWindow>, _dark: bool) -> bool {
    false
}

pub fn hide_title_bar(_window: Pin<&mut QWindow>) -> bool {
    false
}

pub fn show_window_menu(window: Pin<&mut QWindow>, x: f64, y: f64) -> bool {
    // SAFETY: the app's Wayland connection lives as long as the app
    let Some(wayland) = (unsafe { ffi::wayland_application().as_ref() }) else {
        return false;
    };
    let toplevel = ffi::xdg_toplevel(window);
    if toplevel.is_null() {
        return false;
    }
    let Some(library) = &*WAYLAND else {
        return false;
    };
    // xdg_toplevel.show_window_menu(seat, serial, x, y)
    const SHOW_WINDOW_MENU: u32 = 4;
    // SAFETY: the request is sent on the window's own toplevel, with the arguments it takes
    unsafe {
        (library.marshal_flags)(
            toplevel,
            SHOW_WINDOW_MENU,
            core::ptr::null(),
            (library.get_version)(toplevel),
            0,
            wayland.last_input_seat(),
            wayland.last_input_serial(),
            x as i32,
            y as i32,
        );
    }
    true
}
