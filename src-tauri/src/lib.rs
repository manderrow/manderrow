#![deny(unused_must_use)]
#![feature(error_generic_member_access)]
#![feature(exit_status_error)]
#![feature(extend_one)]
#![feature(os_string_truncate)]
#![feature(path_add_extension)]
#![feature(type_changing_struct_update)]

mod commands;
mod game_reviews;
mod games;
mod installing;
mod ipc;
mod launching;
mod mods;
mod paths;
pub mod util;
mod window_state;
mod wrap;

#[cfg(windows)]
mod windows_util;

use std::sync::OnceLock;

use anyhow::{anyhow, Context};
use ipc::IpcState;
use slog::error;

static PRODUCT_NAME: OnceLock<String> = OnceLock::new();
static IDENTIFIER: OnceLock<String> = OnceLock::new();

fn product_name() -> &'static str {
    PRODUCT_NAME.get().unwrap()
}

fn identifier() -> &'static str {
    IDENTIFIER.get().unwrap()
}

#[derive(Debug, Clone, serde::Serialize)]
enum CommandError {
    Aborted,
    Error {
        messages: Vec<String>,
        backtrace: String,
    },
}

impl From<anyhow::Error> for CommandError {
    #[track_caller]
    fn from(value: anyhow::Error) -> Self {
        let backtrace = if value.backtrace().status() != std::backtrace::BacktraceStatus::Disabled {
            value.backtrace().to_string()
        } else {
            std::backtrace::Backtrace::force_capture().to_string()
        };
        Self::Error {
            messages: value.chain().map(|e| e.to_string()).collect(),
            backtrace,
        }
    }
}

impl From<Error> for CommandError {
    fn from(value: Error) -> Self {
        match value {
            Error::Aborted => Self::Aborted,
            Error::Error(e) => Self::from(e),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Aborted by the user")]
    Aborted,
    #[error(transparent)]
    Error(#[from] anyhow::Error),
}

fn run_app(ctx: tauri::Context<tauri::Wry>) -> anyhow::Result<()> {
    let _guard = slog_envlogger::init()?;
    tauri::Builder::default()
        .manage(IpcState::default())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(window_state::init())
        .invoke_handler(tauri::generate_handler![
            commands::close_splashscreen::close_splashscreen,
            commands::games::get_games,
            commands::games::get_games_popularity,
            commands::i18n::get_preferred_locales,
            commands::ipc::send_s2c_message,
            commands::mod_index::fetch_mod_index,
            commands::mod_index::query_mod_index,
            commands::profiles::get_profiles,
            commands::profiles::create_profile,
            commands::profiles::delete_profile,
            commands::profiles::launch_profile,
            commands::profiles::get_profile_mods,
            commands::profiles::install_profile_mod,
            commands::profiles::uninstall_profile_mod,
        ])
        .run(ctx)
        .context("error while running tauri application")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() -> anyhow::Result<()> {
    let ctx = tauri::generate_context!();
    PRODUCT_NAME
        .set(ctx.config().product_name.clone().unwrap())
        .unwrap();
    IDENTIFIER.set(ctx.config().identifier.clone()).unwrap();

    paths::init().unwrap();

    let mut args = std::env::args_os();
    _ = args.next().unwrap();

    match args.next() {
        Some(cmd) if cmd == "wrap" => tauri::async_runtime::block_on(async move {
            match wrap::run(args).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    if cfg!(debug_assertions) {
                        tokio::fs::write("/tmp/manderrow-wrap-crash", &format!("{e:?}")).await?;
                    }
                    Err(e)
                }
            }
        }),
        Some(cmd) => Err(anyhow!("Unrecognized command {cmd:?}")),
        None => run_app(ctx),
    }
}
