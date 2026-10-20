//! Surfaces the app's output on Windows, where it runs as a GUI program without a console of its own.

use core::ffi::{c_char, c_int, c_void};

use windows::core::HSTRING;
use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

// The C runtime's standard streams, which Qt prints its messages to
unsafe extern "C" {
    fn __acrt_iob_func(index: u32) -> *mut c_void;
    fn _fileno(stream: *mut c_void) -> c_int;
    fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
}

/// Writes the app's output to the console it was launched from, if any.
pub fn attach() {
    // SAFETY: this only fails when there is no parent console, in which case there is nothing to do
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_err() {
        return;
    }
    // The C runtime set up its streams before there was a console, so those that weren't
    // redirected write nowhere until they are opened on it. Qt then prints to them, rather than
    // to the debugger, as it sees the console.
    for index in [1, 2] {
        // SAFETY: 1 and 2 are stdout and stderr, which live for as long as the process
        unsafe {
            let stream = __acrt_iob_func(index);
            if _fileno(stream) < 0 {
                freopen(c"CONOUT$".as_ptr(), c"w".as_ptr(), stream);
            }
        }
    }
}

/// Shows an error that ends the app, which would otherwise exit without a trace when launched
/// outside of a console.
pub fn show_fatal_error(message: &str) {
    // SAFETY: the strings outlive the call, and the box has no owner window
    _ = unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(message),
            &HSTRING::from("Manderrow"),
            MB_ICONERROR | MB_OK,
        )
    };
}

/// Shows panics on the main thread, which end the app, after the default hook prints them.
pub fn show_main_thread_panics() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_hook(info);
        if std::thread::current().name() == Some("main") {
            show_fatal_error(&info.to_string());
        }
    }));
}
