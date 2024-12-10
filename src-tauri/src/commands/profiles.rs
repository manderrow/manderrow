use log::error;
use uuid::Uuid;

use crate::paths::DATA_LOCAL_DIR;
use crate::Error;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Profile {
    name: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ProfileWithId {
    id: Uuid,
    #[serde(flatten)]
    metadata: Profile,
}

#[tauri::command]
pub async fn get_profiles(game: &str) -> Result<Vec<ProfileWithId>, Error> {
    let path = DATA_LOCAL_DIR.join("profiles").join(game);
    let mut profiles = Vec::new();
    let mut iter = match tokio::fs::read_dir(path).await {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    while let Some(e) = iter.next_entry().await? {
        let mut path = e.path();
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue
        };
        if id.len() != 36 {
            continue
        }
        let Ok(id) = Uuid::try_parse(id) else {
            continue
        };
        path.push("profile.json");
        let metadata = match tokio::fs::read(&path).await {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(t) => t,
                Err(e) => {
                    error!("Unable to read profile metadata from {path:?}: {e}");
                    continue
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                error!("Unable to read profile metadata from {path:?}: {e}");
                continue
            }
        };
        profiles.push(ProfileWithId { id, metadata });
    }
    Ok(profiles)
}

#[tauri::command]
pub async fn create_profile(game: &str, name: String) -> Result<Uuid, Error> {
    let path = DATA_LOCAL_DIR.join("profiles").join(game);
    tokio::fs::create_dir_all(&path).await?;
    let id = Uuid::new_v4();
    let path = path.join(id.hyphenated().encode_lower(&mut Uuid::encode_buffer()));
    tokio::fs::create_dir(&path).await?;
    tokio::fs::write(path.join("profile.json"), &serde_json::to_vec(&Profile { name })?).await?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_profile(game: &str, id: Uuid) -> Result<(), Error> {
    let path = DATA_LOCAL_DIR.join("profiles").join(game).join(id.hyphenated().encode_lower(&mut Uuid::encode_buffer()));
    tokio::fs::remove_dir_all(&path).await?;
    Ok(())
}