#![deny(unused_must_use)]

mod commands;
mod game_reviews;
mod games;
mod mods;
mod window_state;
mod paths;

use log::error;

#[derive(Debug, Clone, serde::Serialize)]
struct Error {
    message: String,
    backtrace: String,
}

impl<T: std::fmt::Display> From<T> for Error {
    #[track_caller]
    fn from(value: T) -> Self {
        let backtrace = std::backtrace::Backtrace::force_capture();
        error!("{value}\nBacktrace:\n{backtrace}");
        Self {
            message: value.to_string(),
            backtrace: backtrace.to_string(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let level_filter = std::env::var("RUST_LOG")
        .map(|s| {
            s.parse::<log::LevelFilter>()
                .expect("Invalid logging configuration")
        })
        .unwrap_or(log::LevelFilter::Info);
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .filter(move |metadata| {
                    metadata.level() <= level_filter
                        && (metadata.level() < log::Level::Trace
                            || (cfg!(debug_assertions) && metadata.target() == "manderrow"))
                })
                .build(),
        )
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(window_state::init())
        .invoke_handler(tauri::generate_handler![
            commands::close_splashscreen::close_splashscreen,
            commands::games::get_games,
            commands::games::get_games_popularity,
            commands::i18n::get_preferred_locales,
            commands::mod_index::fetch_mod_index,
            commands::mod_index::query_mod_index,
            commands::profiles::create_profile,
            commands::profiles::delete_profile,
            commands::profiles::get_profiles,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
