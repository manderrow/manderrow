use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use log::{error, info};
use tokio::process::Command;
use uuid::Uuid;

use crate::games::GAMES_BY_ID;
use crate::ipc::C2SMessage;
use crate::paths::local_data_dir;
use crate::Error;

pub static PROFILES_DIR: LazyLock<PathBuf> = LazyLock::new(|| local_data_dir().join("profiles"));

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Profile {
    name: String,
    game: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ProfileWithId {
    id: Uuid,
    #[serde(flatten)]
    metadata: Profile,
}

macro_rules! hyphenated_uuid {
    ($id:expr) => {
        $id.hyphenated().encode_lower(&mut Uuid::encode_buffer())
    };
}

#[derive(Debug, thiserror::Error)]
enum ReadProfileError {
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
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
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
            Err(ReadProfileError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => continue,
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
pub async fn launch_profile<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    id: Uuid,
    modded: bool,
) -> Result<(), Error> {
    let mut path = profile_path(id);
    path.push("profile.json");
    let metadata = read_profile_file(&path).await?;
    path.pop();
    let Some(game) = GAMES_BY_ID.get(&*metadata.game).copied() else {
        return Err(format!("Unrecognized game {:?}", metadata.game).into());
    };
    let Some(store_metadata) = game.store_platform_metadata.iter().next() else {
        return Err("Unable to launch game".into());
    };
    let mut command: Command;
    match store_metadata {
        crate::games::StorePlatformMetadata::Steam { store_identifier } => {
            command = Command::new("/Applications/Steam.app/Contents/MacOS/steam_osx");
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

    if modded {
        command.arg("--loader");
        command.arg(game.package_loader.as_str());
    }
    info!("Launching game: {command:?}");
    let status = command.status().await?;
    info!("Launcher exited with status code {status}");

    crate::ipc::spawn_server_listener(&app, c2s_rx)?;

    Ok(())
}
