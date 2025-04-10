mod bep_in_ex;
pub mod commands;

use std::ffi::{OsStr, OsString};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::{anyhow, Context, Result};
use manderrow_paths::cache_dir;
use manderrow_types::games::PackageLoader;
use slog::{debug, info, o};
use tauri::Emitter;
use tauri::{AppHandle, Manager};
use tokio::process::Command;
use uuid::Uuid;

use crate::games::games_by_id;
use crate::ipc::ConnectionId;
use crate::ipc::{C2SMessage, IdentifiedC2SMessage, IpcState};
use crate::profiles::{profile_path, read_profile_file};

pub static LOADERS_DIR: LazyLock<PathBuf> = LazyLock::new(|| cache_dir().join("loaders"));

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum LaunchTarget<'a> {
    #[serde(rename = "profile")]
    Profile(Uuid),
    #[serde(rename = "vanilla")]
    Vanilla(&'a str),
}

pub async fn launch_profile(
    app: AppHandle,
    ipc_state: &IpcState,
    target: LaunchTarget<'_>,
    modded: bool,
    conn_id: ConnectionId,
) -> Result<(), crate::Error> {
    struct Logger {
        app: AssertUnwindSafe<AppHandle>,
        conn_id: ConnectionId,
    }

    impl slog::Drain for Logger {
        type Ok = ();

        type Err = slog::Never;

        fn log(
            &self,
            record: &slog::Record<'_>,
            _values: &slog::OwnedKVList,
        ) -> Result<Self::Ok, Self::Err> {
            _ = self.app.emit_to(
                crate::ipc::EVENT_TARGET,
                crate::ipc::EVENT_NAME,
                IdentifiedC2SMessage {
                    conn_id: self.conn_id,
                    msg: &C2SMessage::Log {
                        level: record.level().into(),
                        scope: "manderrow".into(),
                        message: record.msg().to_string(),
                    },
                },
            );
            Ok(())
        }
    }
    let log = slog::Logger::root(
        Logger {
            app: AssertUnwindSafe(app.clone()),
            conn_id,
        },
        o!(),
    );

    let mut ipc = ipc_state
        .connect(conn_id, app.clone())
        .context("Failed to complete internal IPC connection")?;

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
    enum AgentSource {
        Path(PathBuf),
        Embedded(&'static [u8]),
    }
    let uses_proton = match store_metadata {
        crate::games::StorePlatformMetadata::Steam { .. } => {
            let steam_metadata = game
                .store_platform_metadata
                .iter()
                .find_map(|m| m.steam_or_direct())
                .context("Unsupported store platform")?;

            crate::stores::steam::proton::uses_proton(&log, steam_metadata.id).await?
        }
        _ => false,
    };
    let agent_src = match std::env::var_os("MANDERROW_AGENT_PATH") {
        Some(path) => AgentSource::Path(path.into()),
        None => {
            if uses_proton {
                #[cfg(target_os = "linux")]
                {
                    AgentSource::Embedded(include_bytes!(concat!(
                        std::env!("CARGO_MANIFEST_DIR"),
                        "/../crates/target/x86_64-pc-windows-gnu/release/manderrow_agent.dll"
                    )))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    unreachable!("uses_proton should only be true on Linux")
                }
            } else {
                AgentSource::Path(
                    app.path()
                        .resolve("libmanderrow_agent", tauri::path::BaseDirectory::Resource)
                        .context("Failed to resolve agent path")?,
                )
            }
        }
    };
    match &agent_src {
        AgentSource::Path(path) => debug!(log, "Using bundled agent at {:?}", path),
        AgentSource::Embedded(_) => debug!(log, "Using embedded agent"),
    }
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

            command.arg("{manderrow");

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

                let agent_install_target =
                    crate::stores::steam::paths::resolve_app_install_directory(steam_metadata.id)
                        .await?
                        .join("winhttp.dll");
                match agent_src {
                    AgentSource::Path(agent_path) => {
                        tokio::fs::copy(&agent_path, &agent_install_target)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed to install agent from {:?} at {:?}",
                                    agent_path, agent_install_target
                                )
                            })?;
                    }
                    AgentSource::Embedded(agent_bytes) => {
                        tokio::fs::write(&agent_install_target, agent_bytes)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed to install agent from embedded bytes at {:?}",
                                    agent_install_target
                                )
                            })?;
                    }
                }
            } else {
                crate::stores::steam::launching::ensure_unix_launch_args_are_applied(
                    &log,
                    Some(&mut ipc),
                    steam_metadata.id,
                )
                .await?;

                let AgentSource::Path(agent_path) = agent_src else {
                    unreachable!("embedded is only used when uses_proton is true")
                };
                command.arg("--agent-path");
                command.arg(agent_path);
            }
        }
        _ => return Err(anyhow!("Unsupported game store: {store_metadata:?}").into()),
    }

    command.arg("--enable");

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

    // TODO: find a way to stop this if the launch fails
    let c2s_tx = ipc_state
        .spawn_external(log.clone(), app, conn_id)
        .context("Failed to setup external IPC connection")?;

    command.arg("--c2s-tx");
    command.arg(c2s_tx);

    command.arg("manderrow}");

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
