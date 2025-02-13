use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::importing::thunderstore;
use crate::{CommandError, Reqwest};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Modpack {
    pub name: String,
    pub mods: Vec<ModSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ModSpec {
    /// A mod pulled from somewhere online.
    Online { url: String },
}

#[tauri::command]
pub async fn preview_import_modpack_from_thunderstore_code(
    reqwest: State<'_, Reqwest>,
    thunderstore_id: Uuid,
    profile_id: Option<Uuid>,
) -> Result<Modpack, CommandError> {
    _ = profile_id;
    let profile = thunderstore::lookup_profile(&reqwest, thunderstore_id).await?;

    let mut mods = Vec::with_capacity(profile.manifest.mods.len());

    for m in profile.manifest.mods {
        let (namespace, name) = m.full_name.components();
        mods.push(ModSpec::Online {
            url: format!(
                "https://thunderstore.io/package/download/{namespace}/{name}/{}/",
                m.version
            ),
        });
    }

    Ok(Modpack {
        name: profile.manifest.profile_name,
        mods,
    })
}
