use anyhow::Result;
use manderrow_types::mods::{ModMetadata, ModVersion};
use smol_str::SmolStr;
use uuid::Uuid;

use crate::app::{AppHandle, RawJson};
use crate::{tasks, CommandError, Reqwest};

use super::{Profile, ProfileWithId};

pub async fn get_profiles() -> Result<Vec<ProfileWithId>, CommandError> {
    super::get_profiles().await.map_err(Into::into)
}

pub async fn create_profile(game: SmolStr, name: SmolStr) -> Result<Uuid, CommandError> {
    super::create_profile(game, name).await.map_err(Into::into)
}

pub async fn overwrite_profile_metadata(id: Uuid, metadata: Profile) -> Result<(), CommandError> {
    super::write_profile(id, &metadata)
        .await
        .map_err(anyhow::Error::from)
        .map_err(Into::into)
}

pub async fn delete_profile(id: Uuid) -> Result<(), CommandError> {
    super::delete_profile(id).await.map_err(Into::into)
}

pub async fn get_profile_mods(id: Uuid) -> Result<RawJson, CommandError> {
    super::get_profile_mods(id).await.map_err(Into::into)
}

pub async fn install_profile_mod(
    app: &AppHandle,
    reqwest: &Reqwest,
    id: Uuid,
    r#mod: ModMetadata<'_>,
    version: ModVersion<'_>,
    task_id: tasks::Id,
) -> Result<(), CommandError> {
    super::install_profile_mod(app, reqwest, id, r#mod, version, task_id)
        .await
        .map_err(Into::into)
}

pub async fn uninstall_profile_mod(id: Uuid, owner: &str, name: &str) -> Result<(), CommandError> {
    super::uninstall_profile_mod(id, owner, name)
        .await
        .map_err(Into::into)
}
