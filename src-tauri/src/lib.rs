#![deny(unused_must_use)]
#![feature(error_generic_member_access)]
#![feature(exit_status_error)]
#![feature(extend_one)]
#![feature(file_lock)]
#![feature(os_string_truncate)]
#![feature(path_add_extension)]
#![feature(ptr_as_uninit)]
#![feature(ptr_metadata)]
#![feature(type_alias_impl_trait)]
#![feature(type_changing_struct_update)]
#![feature(unbounded_shifts)]
#![feature(vec_push_within_capacity)]

mod error;
mod games;
mod i18n;
mod importing;
mod installing;
mod ipc;
mod launching;
mod mod_index;
mod mods;
mod paths;
mod profiles;
mod splashscreen;
mod tasks;
mod util;
mod window_state;
mod wrap;

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
        .setup(|app| {
            if !std::env::var_os("TAURI_IMMEDIATE_DEVTOOLS")
                .unwrap_or_default()
                .is_empty()
            {
                let window = app.get_webview_window("main").context("no main window")?;
                window.open_devtools();
            }
            Ok(())
        })
        .manage(IpcState::default())
        .manage(Reqwest(reqwest::Client::builder().build()?))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(window_state::init())
        .invoke_handler(tauri::generate_handler![
            splashscreen::close_splashscreen,
            games::commands::get_games,
            games::commands::search_games,
            games::commands::get_games_popularity,
            games::commands::get_game_mods_downloads,
            i18n::get_preferred_locales,
            importing::commands::preview_import_modpack_from_thunderstore_code,
            importing::commands::import_modpack_from_thunderstore_code,
            installing::commands::clear_cache,
            launching::commands::send_s2c_message,
            launching::commands::launch_profile,
            mod_index::commands::fetch_mod_index,
            mod_index::commands::count_mod_index,
            mod_index::commands::query_mod_index,
            mod_index::commands::get_from_mod_index,
            profiles::commands::get_profiles,
            profiles::commands::create_profile,
            profiles::commands::delete_profile,
            profiles::commands::get_profile_mods,
            profiles::commands::install_profile_mod,
            profiles::commands::uninstall_profile_mod,
            tasks::commands::allocate_task,
            tasks::commands::cancel_task,
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
