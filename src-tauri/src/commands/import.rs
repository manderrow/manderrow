use anyhow::{anyhow, Context};
use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use crate::importing::thunderstore::{self, R2_PROFILE_MANIFEST_FILE_NAME};
use crate::{CommandError, Reqwest};

#[derive(Debug, Clone, Serialize)]
pub struct Modpack {
    pub name: String,
    pub mods: Vec<ModSpec>,
    pub diff: Vec<PathDiff>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ModSpec {
    /// A mod pulled from somewhere online.
    Online { url: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct PathDiff {
    pub path: String,
    pub diff: Diff,
}

#[derive(Debug, Clone, Serialize)]
pub enum Diff {
    Created,
    Deleted,
    Modified,
}

#[tauri::command]
pub async fn preview_import_modpack_from_thunderstore_code(
    reqwest: State<'_, Reqwest>,
    thunderstore_id: Uuid,
    profile_id: Option<Uuid>,
) -> Result<Modpack, CommandError> {
    _ = profile_id;
    let mut profile = thunderstore::lookup_profile(&reqwest, thunderstore_id).await?;

    let mut mods = Vec::with_capacity(profile.manifest.mods.len());

    for m in profile.manifest.mods {
        let (namespace, name) = m.full_name.components();
        mods.push_within_capacity(ModSpec::Online {
            url: format!(
                "https://thunderstore.io/package/download/{namespace}/{name}/{}/",
                m.version
            ),
        })
        .unwrap();
    }

    // exclude the manifest file
    let mut diff = Vec::with_capacity(profile.archive.len() - 1);

    for _ in 0..diff.capacity() {
        let file = profile
            .archive
            .by_index(diff.len())
            .context("Failed to open file in archive")?;
        if file.name() == R2_PROFILE_MANIFEST_FILE_NAME {
            continue;
        }
        let path = file
            .enclosed_name()
            .with_context(|| format!("File in archive has a bad path: {:?}", file.name()))?
            .into_os_string()
            .into_string()
            .map_err(|s| anyhow!("Path must be valid Unicode: {s:?}"))?;

        diff.push_within_capacity(PathDiff {
            path,
            diff: Diff::Created,
        })
        .unwrap();
    }

    Ok(Modpack {
        name: profile.manifest.profile_name,
        mods,
        diff,
    })
}
