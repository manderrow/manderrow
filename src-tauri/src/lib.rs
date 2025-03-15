#![deny(unused_must_use)]
#![feature(error_generic_member_access)]
#![feature(exit_status_error)]
#![feature(extend_one)]
#![feature(os_string_truncate)]
#![feature(path_add_extension)]
#![feature(ptr_as_uninit)]
#![feature(ptr_metadata)]
#![feature(type_alias_impl_trait)]
#![feature(type_changing_struct_update)]
#![feature(unbounded_shifts)]

mod commands;
mod error;
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

use std::{ops::Deref, sync::OnceLock};

use anyhow::{anyhow, Context};
use ipc::IpcState;

pub use error::{CommandError, Error};
use tauri::Manager;

static PRODUCT_NAME: OnceLock<String> = OnceLock::new();
static IDENTIFIER: OnceLock<String> = OnceLock::new();

fn product_name() -> &'static str {
    PRODUCT_NAME.get().unwrap()
}

fn identifier() -> &'static str {
    IDENTIFIER.get().unwrap()
}

struct Reqwest(reqwest::Client);

impl Deref for Reqwest {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn run_app(ctx: tauri::Context<tauri::Wry>) -> anyhow::Result<()> {
    let _guard = slog_envlogger::init()?;
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let window = app.get_webview_window("main").expect("no main window");

            window.unminimize().ok();
            window.set_focus().ok();
        }))
        .manage(IpcState::default())
        .manage(Reqwest(reqwest::Client::builder().build()?))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(window_state::init())
        .invoke_handler(tauri::generate_handler![
            commands::close_splashscreen::close_splashscreen,
            commands::games::get_games,
            commands::games::get_games_popularity,
            commands::games::get_game_mods_downloads,
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
    if cfg!(target_os = "linux") {
        // Only provide a default value, don't override the user's choice.
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            // Fixes an intermitent issue on Wayland where the window freezes after resizing.
            // Known to occur with NVIDIA proprietary drivers, untested under other conditions.
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
    }

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
