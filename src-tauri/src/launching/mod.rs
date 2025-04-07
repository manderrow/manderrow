mod bep_in_ex;
pub mod commands;

use std::ffi::{OsStr, OsString};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::{anyhow, Context, Result};
use manderrow_paths::cache_dir;
use manderrow_types::games::PackageLoader;
use slog::{info, o};
use tauri::{ipc::Channel, AppHandle};
use tokio::process::Command;
use uuid::Uuid;

use crate::games::games_by_id;
use crate::ipc::{InProcessIpcStateExt, S2CMessage};
use crate::profiles::{profile_path, read_profile_file};
use crate::util::hyphenated_uuid;
use crate::{
    ipc::{C2SMessage, IpcState},
    CommandError,
};

pub static LOADERS_DIR: LazyLock<PathBuf> = LazyLock::new(|| cache_dir().join("loaders"));

pub async fn send_s2c_message(ipc_state: &IpcState, msg: S2CMessage) -> Result<()> {
    ipc_state
        .s2c_tx
        .send_async(msg)
        .await
        .context("Failed to send IPC message")?;
    Ok(())
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum LaunchTarget<'a> {
    #[serde(rename = "profile")]
    Profile(Uuid),
    #[serde(rename = "vanilla")]
    Vanilla(&'a str),
}

pub async fn launch_profile(
    app_handle: AppHandle,
    ipc_state: &IpcState,
    target: LaunchTarget<'_>,
    modded: bool,
    channel: Channel<C2SMessage>,
) -> Result<(), CommandError> {
    struct Logger {
        c2s_tx: AssertUnwindSafe<Channel<C2SMessage>>,
    }

    impl slog::Drain for Logger {
        type Ok = ();

        type Err = slog::Never;

        fn log(
            &self,
            record: &slog::Record<'_>,
            _values: &slog::OwnedKVList,
        ) -> Result<Self::Ok, Self::Err> {
            _ = tokio::task::block_in_place(|| {
                self.c2s_tx.send(C2SMessage::Log {
                    level: record.level().into(),
                    scope: "manderrow".into(),
                    message: record.msg().to_string(),
                })
            });
            Ok(())
        }
    }
    let log = slog::Logger::root(
        Logger {
            c2s_tx: AssertUnwindSafe(channel.clone()),
        },
        o!(),
    );

    let game = match target {
        LaunchTarget::Profile(id) => {
            let mut path = profile_path(id);
            path.push("profile.json");
            let metadata = read_profile_file(&path)
                .await
                .context("Failed to read profile")?;
            path.pop();
            games_by_id()?
                .get(&*metadata.game)
                .copied()
                .with_context(|| format!("Unrecognized game {:?}", metadata.game))?
        }
        LaunchTarget::Vanilla(id) => games_by_id()?
            .get(id)
            .copied()
            .with_context(|| format!("Unrecognized game {:?}", id))?,
    };
    let Some(store_metadata) = game.store_platform_metadata.iter().next() else {
        return Err(anyhow!("Unable to launch game").into());
    };
    let mut command: Command;
    match store_metadata {
        crate::games::StorePlatformMetadata::Steam {
            store_identifier, ..
        } => {
            let steam_metadata = game
                .store_platform_metadata
                .iter()
                .find_map(|m| m.steam_or_direct())
                .context("Unsupported store platform")?;

            let uses_proton =
                crate::stores::steam::proton::uses_proton(&log, steam_metadata.id).await?;

            command = if cfg!(windows) {
                #[cfg(windows)]
                {
                    let mut p =
                        crate::stores::steam::paths::get_steam_install_path_from_registry()?;
                    p.push("steam.exe");
                    Command::new(p)
                }
                #[cfg(not(windows))]
                unreachable!()
            } else if cfg!(target_os = "macos") {
                Command::new("/Applications/Steam.app/Contents/MacOS/steam_osx")
            } else if cfg!(unix) {
                Command::new("steam")
            } else {
                return Err(anyhow!("Unsupported platform for Steam").into());
            };
            command.arg("-applaunch").arg(&**store_identifier);

            if cfg!(windows) || uses_proton {
                if uses_proton {
                    // TODO: don't overwrite anything without checking with the user
                    //       via a doctor's note.
                    crate::stores::steam::proton::ensure_wine_will_load_dll_override(
                        &log,
                        steam_metadata.id,
                        "winhttp",
                    )
                    .await?;
                }

                todo!("install agent winhttp.dll or other proxy");
                // let doorstop_install_target =
                //     resolve_steam_app_install_directory(steam_metadata.id)
                //         .await?
                //         .join("winhttp.dll");
                // if let Some(doorstop_path) = doorstop_path {
                //     tokio::fs::copy(doorstop_path, &doorstop_install_target).await?;
                // } else {
                //     install_file(
                //         // TODO: communicate via IPC
                //         None,
                //         log,
                //         &Reqwest(reqwest::Client::new()),
                //         doorstop_url,
                //         // suffix is unnecessary here
                //         Some(crate::installing::CacheOptions::by_hash(doorstop_hash)),
                //         &doorstop_install_target,
                //         None,
                //     )
                //     .await?;
                // }

                command.arg("{manderrow");
            } else {
                crate::stores::steam::launching::ensure_launch_args_are_applied(
                    &log,
                    Some(ipc_state.bi(&channel)),
                    game.id,
                    steam_metadata.id,
                )
                .await?;

                command.arg("{manderrow");

                // this is a very special arg that is handled and removed by the wrapper
                if let Some(path) = std::env::var_os("MANDERROW_AGENT_PATH") {
                    command.arg("--agent-path");
                    command.arg(path);
                }
            }
        }
        _ => return Err(anyhow!("Unsupported game store: {store_metadata:?}").into()),
    }

    let (c2s_rx, c2s_tx) = manderrow_ipc::ipc_channel::ipc::IpcOneShotServer::<C2SMessage>::new()
        .context("Failed to create IPC channel")?;

    command.arg("--enable");

    command.arg("--c2s-tx");
    command.arg(c2s_tx);

    if let LaunchTarget::Profile(id) = target {
        command.arg("--profile");
        command.arg(hyphenated_uuid!(id));
    }

    if modded {
        match (target, game.package_loader) {
            (LaunchTarget::Vanilla(_), _) => {}
            (LaunchTarget::Profile(profile), PackageLoader::BepInEx) => {
                crate::launching::bep_in_ex::emit_instructions(
                    &log,
                    InstructionEmitter {
                        command: &mut command,
                    },
                    game.id,
                    profile,
                    std::env::var_os("OVERRIDE_DOORSTOP_LIBRARY_PATH").map(PathBuf::from),
                )
                .await?;
            }
            (_, loader) => {
                return Err(anyhow!("The mod loader {loader:?} is not yet supported").into())
            }
        }
    }

    command.arg("manderrow}");

    // TODO: find a way to stop this if the launch fails
    crate::ipc::spawn_c2s_pipe(log.clone(), app_handle, channel, c2s_rx)?;

    info!(log, "Launching game: {command:?}");
    let status = command
        .status()
        .await
        .context("Failed to wait for subprocess to exit")?;
    info!(log, "Launcher exited with status code {status}");

    Ok(())
}

struct InstructionEmitter<'a> {
    command: &'a mut Command,
}

impl<'a> InstructionEmitter<'a> {
    pub fn load_library(&mut self, path: impl Into<OsString>) {
        self.command
            .args(["--insn-load-library".into(), path.into()]);
    }

    pub fn set_var(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        let mut kv = key.as_ref().to_owned();
        kv.push("=");
        kv.push(value.as_ref());
        self.command.args(["--insn-set-var".into(), kv]);
    }

    pub fn prepend_arg(&mut self, arg: impl Into<OsString>) {
        self.command.args(["--insn-prepend-arg".into(), arg.into()]);
    }

    pub fn append_arg(&mut self, arg: impl Into<OsString>) {
        self.command.args(["--insn-append-arg".into(), arg.into()]);
    }
}
