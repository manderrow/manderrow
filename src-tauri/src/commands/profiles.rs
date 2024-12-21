use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Context as _;
use futures::stream::FuturesOrdered;
use futures::StreamExt as _;
use log::{error, info};
use tauri::ipc::Channel;
use tokio::process::Command;
use uuid::Uuid;

use crate::games::{PackageLoader, GAMES_BY_ID};
use crate::installing::{install_zip, uninstall_package};
use crate::ipc::C2SMessage;
use crate::launching::bep_in_ex::BEP_IN_EX_FOLDER;
use crate::mods::{Mod, ModAndVersion};
use crate::paths::local_data_dir;
use crate::util::IoErrorKindExt as _;
use crate::Error;

pub static PROFILES_DIR: LazyLock<PathBuf> = LazyLock::new(|| local_data_dir().join("profiles"));

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Profile {
    pub name: String,
    pub game: String,
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
pub async fn get_profiles() -> Result<Vec<ProfileWithId>, Error> {
    let mut profiles = Vec::new();
    let mut iter = match tokio::fs::read_dir(&*PROFILES_DIR).await {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    while let Some(e) = iter.next_entry().await? {
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
                error!("Unable to read profile metadata from {path:?}: {e}");
                continue;
            }
        };
        profiles.push(ProfileWithId { id, metadata });
    }
    Ok(profiles)
}

#[tauri::command]
pub async fn create_profile(game: String, name: String) -> Result<Uuid, Error> {
    tokio::fs::create_dir_all(&*PROFILES_DIR).await?;
    let id = Uuid::new_v4();
    let mut path = profile_path(id);
    tokio::fs::create_dir(&path).await?;
    path.push("profile.json");
    tokio::fs::write(path, &serde_json::to_vec(&Profile { name, game })?).await?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_profile(id: Uuid) -> Result<(), Error> {
    let path = profile_path(id);
    tokio::fs::remove_dir_all(&path).await?;
    Ok(())
}

#[tauri::command]
pub async fn launch_profile(
    id: Uuid,
    modded: bool,
    channel: Channel<C2SMessage>,
) -> Result<(), Error> {
    let mut path = profile_path(id);
    path.push("profile.json");
    let metadata = read_profile_file(&path).await?;
    path.pop();
    let Some(game) = GAMES_BY_ID.get(&*metadata.game).copied() else {
        return Err(Error::new(format_args!(
            "Unrecognized game {:?}",
            metadata.game
        )));
    };
    let Some(store_metadata) = game.store_platform_metadata.iter().next() else {
        return Err("Unable to launch game".into());
    };
    let mut command: Command;
    match store_metadata {
        crate::games::StorePlatformMetadata::Steam { store_identifier } => {
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
                return Err("Unsupported platform for Steam".into());
            };
            command.arg("-applaunch").arg(store_identifier);
        }
        _ => return Err("Unsupported game store".into()),
    }

    let (c2s_rx, c2s_tx) = ipc_channel::ipc::IpcOneShotServer::<C2SMessage>::new()?;

    command.arg(";");
    command.arg("--c2s-tx");
    command.arg(c2s_tx);
    command.arg("--profile");
    command.arg(hyphenated_uuid!(id));

    if let Some(path) = std::env::var_os("MANDERROW_WRAPPER_STAGE2_PATH") {
        command.arg("--wrapper-stage2");
        command.arg(path);
    }

    if modded {
        command.arg("--loader");
        command.arg(game.package_loader.as_str());
    }
    info!("Launching game: {command:?}");
    let status = command.status().await?;
    info!("Launcher exited with status code {status}");

    crate::ipc::spawn_server_listener(channel, c2s_rx)?;

    Ok(())
}

const MANIFEST_FILE_NAME: &str = "manderrow_mod.json";

#[tauri::command]
pub async fn get_profile_mods(id: Uuid) -> Result<Vec<ModAndVersion>, Error> {
    let mut path = profile_path(id);
    path.push("profile.json");
    let metadata = read_profile_file(&path).await?;
    path.pop();

    let game = GAMES_BY_ID.get(&*metadata.game).context("No such game")?;

    match game.package_loader {
        PackageLoader::BepInEx => {
            path.push(BEP_IN_EX_FOLDER);
            path.push("BepInEx");
            path.push("plugins");

            let mut iter = match tokio::fs::read_dir(&path).await {
                Ok(t) => t,
                Err(e) if e.is_not_found() => return Ok(Vec::new()),
                Err(e) => return Err(e.into()),
            };
            let mut tasks = FuturesOrdered::new();
            while let Some(e) = iter.next_entry().await? {
                if e.file_type().await?.is_dir() {
                    let mut path = path.clone();
                    tasks.push_back(tokio::task::spawn(async move {
                        path.push(e.file_name());
                        path.push(MANIFEST_FILE_NAME);
                        tokio::task::block_in_place(|| {
                            Ok::<_, Error>(Some(serde_json::from_reader(std::io::BufReader::new(
                                match std::fs::File::open(&path) {
                                    Ok(t) => t,
                                    Err(e) if e.is_not_found() => {
                                        return Ok(None)
                                    }
                                    Err(e) => return Err(e.into()),
                                },
                            ))?))
                        })
                    }));
                }
            }
            let mut buf = Vec::new();
            while let Some(r) = tasks.next().await {
                if let Some(m) = r?? {
                    buf.push(m);
                }
            }
            Ok(buf)
        }
        _ => Ok(Vec::new()),
    }
}

#[tauri::command]
pub async fn install_profile_mod(id: Uuid, r#mod: Mod, version: usize) -> Result<(), Error> {
    let mut path = profile_path(id);
    path.push("profile.json");
    let metadata = read_profile_file(&path).await?;
    path.pop();

    let game = GAMES_BY_ID.get(&*metadata.game).context("No such game")?;

    let mod_with_version = ModAndVersion {
        r#mod: r#mod.metadata,
        game: metadata.game,
        version: r#mod
            .versions
            .into_iter()
            .nth(version)
            .ok_or("No such version")?,
    };

    match game.package_loader {
        PackageLoader::BepInEx => {
            path.push(BEP_IN_EX_FOLDER);
            path.push("BepInEx");
            path.push("plugins");

            tokio::fs::create_dir_all(&path).await?;

            path.push(&mod_with_version.r#mod.full_name);
            let staged = install_zip(&mod_with_version.version.download_url, None, &path).await?;

            tokio::task::block_in_place(|| {
                serde_json::to_writer(
                    std::io::BufWriter::new(std::fs::File::create(
                        staged.path().join(MANIFEST_FILE_NAME),
                    )?),
                    &mod_with_version,
                )?;
                Ok::<_, anyhow::Error>(())
            })?;

            staged.finish().await?;

            Ok(())
        }
        _ => Err("Unsupported package loader for mod installation".into()),
    }
}

#[tauri::command]
pub async fn uninstall_profile_mod(id: Uuid, mod_name: &str) -> Result<(), Error> {
    let mut path = profile_path(id);
    path.push("profile.json");
    let metadata = read_profile_file(&path).await?;
    path.pop();

    let game = GAMES_BY_ID.get(&*metadata.game).context("No such game")?;

    match game.package_loader {
        PackageLoader::BepInEx => {
            path.push(BEP_IN_EX_FOLDER);
            path.push("BepInEx");
            path.push("plugins");
            path.push(mod_name);

            // remove the manifest so it isn't left over after uninstalling the package
            path.push(MANIFEST_FILE_NAME);
            tokio::fs::remove_file(&path).await?;
            path.pop();

            // keep_changes is true so that configs and any other changes are
            // preserved. Zero-risk uninstallation!
            uninstall_package(&path, true).await?;
            Ok(())
        }
        _ => Err("Unsupported package loader for mod installation".into()),
    }
}
