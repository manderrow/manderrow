#![deny(unused_must_use)]

mod commands;
pub mod games;
pub mod mods;

#[derive(Debug, Clone, serde::Serialize)]
struct Error {
    message: String,
    backtrace: String,
}

impl<T: std::fmt::Display> From<T> for Error {
    #[track_caller]
    fn from(value: T) -> Self {
        let backtrace = std::backtrace::Backtrace::force_capture();
        println!("{value}\nBacktrace:\n{backtrace}");
        Self {
            message: value.to_string(),
            backtrace: backtrace.to_string(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::games::get_games,
            commands::mod_index::fetch_mod_index,
            commands::mod_index::query_mod_index
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
