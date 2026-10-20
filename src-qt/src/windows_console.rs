//! Surfaces the app's output on Windows, where it runs as a GUI program without a console of its own.

use windows::core::HSTRING;
use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

/// Writes the app's output to the console it was launched from, if any.
pub fn attach() {
    // SAFETY: this only fails when there is no parent console, in which case there is nothing to do
    _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
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
