pub mod bep_in_ex;
pub mod commands;

use std::sync::LazyLock;
use std::{panic::AssertUnwindSafe, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use slog::{info, o};
use tauri::{ipc::Channel, AppHandle};
use tokio::process::Command;
use uuid::Uuid;

use crate::games::games_by_id;
use crate::ipc::S2CMessage;
use crate::profiles::{profile_path, read_profile, read_profile_file};
use crate::util::hyphenated_uuid;
use crate::{
    ipc::{C2SMessage, IpcState},
    paths::cache_dir,
    CommandError,
};

pub static LOADERS_DIR: LazyLock<PathBuf> = LazyLock::new(|| cache_dir().join("loaders"));

pub async fn send_s2c_message(ipc_state: &IpcState, msg: S2CMessage) -> Result<()> {
    let s2c_tx = ipc_state.s2c_tx.read().await;
    if let Some(s2c_tx) = &*s2c_tx {
        s2c_tx
            .send(msg)
            .await
            .context("Failed to send IPC message")?;
    }
    Ok(())
}

pub async fn launch_profile(
    app_handle: AppHandle,
    ipc_state: &IpcState,
    id: Uuid,
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

    let mut path = profile_path(id);
    path.push("profile.json");
    let metadata = read_profile_file(&path)
        .await
        .context("Failed to read profile")?;
    path.pop();
    let Some(game) = games_by_id()?.get(&*metadata.game).copied() else {
        return Err(anyhow!("Unrecognized game {:?}", metadata.game).into());
    };
    let Some(store_metadata) = game.store_platform_metadata.iter().next() else {
        return Err(anyhow!("Unable to launch game").into());
    };
    let mut command: Command;
    match store_metadata {
        crate::games::StorePlatformMetadata::Steam {
            store_identifier, ..
        } => {
            let profile = read_profile(id).await.context("Failed to read profile")?;
            let steam_metadata = games_by_id()?
                .get(&*profile.game)
                .context("No such game")?
                .store_platform_metadata
                .iter()
                .find_map(|m| m.steam_or_direct())
                .context("Unsupported store platform")?;

            crate::stores::steam::launching::ensure_launch_args_are_applied(
                &log,
                Some(ipc_state.spc(channel.clone())),
                game.id,
                steam_metadata.id,
            )
            .await?;

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
        }
        _ => return Err(anyhow!("Unsupported game store: {store_metadata:?}").into()),
    }

    let (c2s_rx, c2s_tx) = ipc_channel::ipc::IpcOneShotServer::<C2SMessage>::new()
        .context("Failed to create IPC channel")?;

    command.arg(";");
    command.arg("--c2s-tx");
    command.arg(c2s_tx);
    command.arg("--profile");
    command.arg(hyphenated_uuid!(id));

    // TODO: use Tauri sidecar
    if let Some(path) = std::env::var_os("MANDERROW_WRAPPER_STAGE2_PATH") {
        command.arg("--wrapper-stage2");
        command.arg(path);
    }

    if let Some(path) = std::env::var_os("OVERRIDE_DOORSTOP_LIBRARY_PATH") {
        command.arg("--doorstop-path");
        command.arg(path);
    }

    if modded {
        command.arg("--loader");
        command.arg(game.package_loader.as_str());
    }

    command.arg(";");

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
