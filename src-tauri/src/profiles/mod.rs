pub mod commands;

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use futures::stream::FuturesOrdered;
use futures::StreamExt as _;
use slog::error;
use smol_str::SmolStr;
use tauri::AppHandle;
use uuid::Uuid;

use crate::installing::{install_zip, uninstall_package};
use crate::mods::{ModAndVersion, ModMetadata, ModVersion};
use crate::paths::local_data_dir;
use crate::util::{hyphenated_uuid, IoErrorKindExt as _};
use crate::Reqwest;

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

#[derive(Debug, thiserror::Error)]
pub enum ReadProfileError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Decoding(#[from] serde_json::Error),
}

pub async fn read_profile_file(path: &Path) -> Result<Profile, ReadProfileError> {
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

pub async fn get_profiles() -> Result<Vec<ProfileWithId>> {
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

pub async fn create_profile(game: SmolStr, name: SmolStr) -> Result<Uuid> {
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

pub async fn delete_profile(id: Uuid) -> Result<()> {
    let path = profile_path(id);
    tokio::fs::remove_dir_all(&path)
        .await
        .context("Failed to delete profile directory")?;
    Ok(())
}

pub const MODS_FOLDER: &str = "mods";

const MANIFEST_FILE_NAME: &str = "manderrow_mod.json";

pub async fn get_profile_mods(id: Uuid) -> Result<tauri::ipc::Response> {
    let mut path = profile_path(id);

    path.push(MODS_FOLDER);

    let mut iter = match tokio::fs::read_dir(&path).await {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(tauri::ipc::Response::new("[]".to_owned())),
        Err(e) => return Err(anyhow::Error::from(e).into()),
    };
    let mut tasks = FuturesOrdered::new();
    while let Some(e) = iter.next_entry().await.map_err(anyhow::Error::from)? {
        if e.file_type().await.map_err(anyhow::Error::from)?.is_dir() {
            let mut path = path.clone();
            tasks.push_back(tokio::task::spawn(async move {
                path.push(e.file_name());
                path.push(MANIFEST_FILE_NAME);
                match tokio::fs::read_to_string(&path).await {
                    Ok(t) => Ok(Some(t)),
                    Err(e) if e.is_not_found() => return Ok(None),
                    Err(e) => {
                        return Err(anyhow::Error::from(e)
                            .context(format!("Failed to read mod manifest {path:?}")))
                    }
                }
            }));
        }
    }
    let mut buf = "[".to_owned();
    let mut first = true;
    while let Some(r) = tasks.next().await {
        if let Some(m) = r.map_err(anyhow::Error::from)?? {
            if first {
                first = false;
            } else {
                buf.push(',');
            }
            buf.push_str(&m);
        }
    }
    buf.push(']');
    Ok(tauri::ipc::Response::new(buf))
}

pub async fn install_profile_mod(
    app: &AppHandle,
    reqwest: &Reqwest,
    id: Uuid,
    r#mod: ModMetadata<'_>,
    version: ModVersion<'_>,
) -> Result<()> {
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
        Some(app),
        &log,
        reqwest,
        &format!(
            "https://gcdn.thunderstore.io/live/repository/packages/{}-{}-{}.zip",
            r#mod.owner, r#mod.name, version.version_number
        ),
        Some(crate::installing::CacheOptions::ByUrl),
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

pub async fn uninstall_profile_mod(id: Uuid, owner: &str, name: &str) -> Result<()> {
    let log = slog_scope::logger();

    let mut path = profile_path(id);

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
