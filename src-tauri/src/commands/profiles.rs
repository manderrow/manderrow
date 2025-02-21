use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{anyhow, Context as _, Result};
use futures::stream::FuturesOrdered;
use futures::StreamExt as _;
use slog::{error, info, o};
use smol_str::SmolStr;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tokio::process::Command;
use uuid::Uuid;

use crate::games::{PackageLoader, GAMES_BY_ID};
use crate::installing::{install_zip, uninstall_package};
use crate::ipc::{C2SMessage, IpcState};
use crate::mods::{ModAndVersion, ModMetadata, ModVersion};
use crate::paths::local_data_dir;
use crate::util::IoErrorKindExt as _;
use crate::{CommandError, Reqwest};

pub static PROFILES_DIR: LazyLock<PathBuf> = LazyLock::new(|| local_data_dir().join("profiles"));

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Profile {
    pub name: SmolStr,
    pub game: SmolStr,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ProfileWithId {
    pub id: Uuid,
    #[serde(flatten)]
    pub metadata: Profile,
}

macro_rules! hyphenated_uuid {
    ($id:expr) => {
        $id.hyphenated().encode_lower(&mut Uuid::encode_buffer())
    };
}

#[derive(Debug, thiserror::Error)]
pub enum ReadProfileError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Decoding(#[from] serde_json::Error),
}

async fn read_profile_file(path: &Path) -> Result<Profile, ReadProfileError> {
    Ok(serde_json::from_slice(&tokio::fs::read(path).await?)?)
}

pub async fn read_profile(id: Uuid) -> Result<Profile, ReadProfileError> {
    let mut path = profile_path(id);
    path.push("profile.json");
    read_profile_file(&path).await
}

pub fn profile_path(id: Uuid) -> PathBuf {
    PROFILES_DIR.join(hyphenated_uuid!(id))
}

#[tauri::command]
pub async fn get_profiles() -> Result<Vec<ProfileWithId>, CommandError> {
    let log = slog_scope::logger();

    let mut profiles = Vec::new();
    let mut iter = match tokio::fs::read_dir(&*PROFILES_DIR).await {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(Vec::new()),
        Err(e) => return Err(e).context("Failed to read profiles directory")?,
    };
    while let Some(e) = iter
        .next_entry()
        .await
        .context("Failed to read profiles directory")?
    {
        let mut path = e.path();
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if id.len() != 36 {
            continue;
        }
        let Ok(id) = Uuid::try_parse(id) else {
            continue;
        };
        path.push("profile.json");
        let metadata = match read_profile_file(&path).await {
            Ok(t) => t,
            Err(ReadProfileError::Io(e)) if e.is_not_found() => continue,
            Err(e) => {
                error!(log, "Unable to read profile metadata from {path:?}: {e}");
                continue;
            }
        };
        profiles.push(ProfileWithId { id, metadata });
    }
    Ok(profiles)
}

#[tauri::command]
pub async fn create_profile(game: SmolStr, name: SmolStr) -> Result<Uuid, CommandError> {
    tokio::fs::create_dir_all(&*PROFILES_DIR)
        .await
        .context("Failed to create profiles directory")?;
    let id = Uuid::new_v4();
    let mut path = profile_path(id);
    tokio::fs::create_dir(&path)
        .await
        .context("Failed to create profile directory")?;
    path.push("profile.json");
    tokio::fs::write(path, &serde_json::to_vec(&Profile { name, game }).unwrap())
        .await
        .context("Failed to write profile metadata")?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_profile(id: Uuid) -> Result<(), CommandError> {
    let path = profile_path(id);
    tokio::fs::remove_dir_all(&path)
        .await
        .context("Failed to delete profile directory")?;
    Ok(())
}

pub const MODS_FOLDER: &str = "mods";

#[tauri::command]
pub async fn launch_profile(
    app_handle: AppHandle,
    ipc_state: State<'_, IpcState>,
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
    let Some(game) = GAMES_BY_ID.get(&*metadata.game).copied() else {
        return Err(anyhow!("Unrecognized game {:?}", metadata.game).into());
    };
    let Some(store_metadata) = game.store_platform_metadata.iter().next() else {
        return Err(anyhow!("Unable to launch game").into());
    };
    let mut command: Command;
    match store_metadata {
        crate::games::StorePlatformMetadata::Steam { store_identifier } => {
            let profile = read_profile(id).await.context("Failed to read profile")?;
            let steam_id = GAMES_BY_ID
                .get(&*profile.game)
                .context("No such game")?
                .store_platform_metadata
                .iter()
                .find_map(|m| m.steam_or_direct())
                .context("Unsupported store platform")?;

            crate::launching::steam::ensure_launch_args_are_applied(
                &log,
                Some(ipc_state.spc(channel.clone())),
                steam_id,
            )
            .await?;

            command = if cfg!(windows) {
                #[cfg(windows)]
                {
                    let mut p =
                        crate::launching::steam::paths::get_steam_install_path_from_registry()?;
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
            command.arg("-applaunch").arg(store_identifier);
        }
        _ => return Err(anyhow!("Unsupported game store").into()),
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

const MANIFEST_FILE_NAME: &str = "manderrow_mod.json";

#[tauri::command]
pub async fn get_profile_mods(id: Uuid) -> Result<Vec<ModAndVersion>, CommandError> {
    let mut path = profile_path(id);
    path.push("profile.json");
    let metadata = read_profile_file(&path)
        .await
        .context("Failed to read profile")?;
    path.pop();

    let game = GAMES_BY_ID.get(&*metadata.game).context("No such game")?;

    match game.package_loader {
        PackageLoader::BepInEx => {
            path.push(MODS_FOLDER);

            let mut iter = match tokio::fs::read_dir(&path).await {
                Ok(t) => t,
                Err(e) if e.is_not_found() => return Ok(Vec::new()),
                Err(e) => return Err(anyhow::Error::from(e).into()),
            };
            let mut tasks = FuturesOrdered::new();
            while let Some(e) = iter.next_entry().await.map_err(anyhow::Error::from)? {
                if e.file_type().await.map_err(anyhow::Error::from)?.is_dir() {
                    let mut path = path.clone();
                    tasks.push_back(tokio::task::spawn(async move {
                        path.push(e.file_name());
                        path.push(MANIFEST_FILE_NAME);
                        tokio::task::block_in_place(|| {
                            Ok::<_, anyhow::Error>(Some(serde_json::from_reader(
                                std::io::BufReader::new(match std::fs::File::open(&path) {
                                    Ok(t) => t,
                                    Err(e) if e.is_not_found() => return Ok(None),
                                    Err(e) => return Err(e.into()),
                                }),
                            )?))
                        })
                    }));
                }
            }
            let mut buf = Vec::new();
            while let Some(r) = tasks.next().await {
                if let Some(m) = r.map_err(anyhow::Error::from)?? {
                    buf.push(m);
                }
            }
            Ok(buf)
        }
        _ => Ok(Vec::new()),
    }
}

#[tauri::command]
pub async fn install_profile_mod(
    reqwest: State<'_, Reqwest>,
    id: Uuid,
    r#mod: ModMetadata,
    version: ModVersion,
) -> Result<(), CommandError> {
    let log = slog_scope::logger();

    let mut path = profile_path(id);

    path.push(MODS_FOLDER);

    tokio::fs::create_dir_all(&path)
        .await
        .context("Failed to create mods directory")?;

    path.push(&r#mod.owner);
    path.as_mut_os_string().push("-");
    path.as_mut_os_string().push(&r#mod.name);
    let staged = install_zip(
        &log,
        &*reqwest,
        &format!(
            "https://thunderstore.io/package/download/{}/{}/{}/",
            r#mod.owner, r#mod.name, version.version_number
        ),
        None,
        &path,
    )
    .await?;

    tokio::task::block_in_place(|| {
        serde_json::to_writer(
            std::io::BufWriter::new(std::fs::File::create(
                staged.path().join(MANIFEST_FILE_NAME),
            )?),
            &ModAndVersion {
                r#mod: r#mod,
                version,
            },
        )?;
        Ok::<_, anyhow::Error>(())
    })?;

    staged.finish(&log).await?;

    Ok(())
}

#[tauri::command]
pub async fn uninstall_profile_mod(id: Uuid, owner: &str, name: &str) -> Result<(), CommandError> {
    let log = slog_scope::logger();

    let mut path = profile_path(id);
    path.push("profile.json");
    let metadata = read_profile_file(&path)
        .await
        .context("Failed to read profile")?;
    path.pop();

    let game = GAMES_BY_ID.get(&*metadata.game).context("No such game")?;

    match game.package_loader {
        PackageLoader::BepInEx => {
            path.push(MODS_FOLDER);
            path.push(owner);
            path.as_mut_os_string().push("-");
            path.as_mut_os_string().push(name);

            // remove the manifest so it isn't left over after uninstalling the package
            path.push(MANIFEST_FILE_NAME);
            tokio::fs::remove_file(&path)
                .await
                .context("Failed to remove manifest file")?;
            path.pop();

            // keep_changes is true so that configs and any other changes are
            // preserved. Zero-risk uninstallation!
            uninstall_package(&log, &path, true).await?;
            Ok(())
        }
        _ => Err(anyhow!("Unsupported package loader for mod installation").into()),
    }
}
