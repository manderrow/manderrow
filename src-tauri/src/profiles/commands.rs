use anyhow::Result;
use smol_str::SmolStr;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::mods::{ModMetadata, ModVersion};
use crate::{CommandError, Reqwest};

use super::ProfileWithId;

#[tauri::command]
pub async fn get_profiles() -> Result<Vec<ProfileWithId>, CommandError> {
    super::get_profiles().await.map_err(Into::into)
}

#[tauri::command]
pub async fn create_profile(game: SmolStr, name: SmolStr) -> Result<Uuid, CommandError> {
    super::create_profile(game, name).await.map_err(Into::into)
}

#[tauri::command]
pub async fn delete_profile(id: Uuid) -> Result<(), CommandError> {
    super::delete_profile(id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn get_profile_mods(id: Uuid) -> Result<tauri::ipc::Response, CommandError> {
    super::get_profile_mods(id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn install_profile_mod(
    app: AppHandle,
    reqwest: State<'_, Reqwest>,
    id: Uuid,
    r#mod: ModMetadata<'_>,
    version: ModVersion<'_>,
) -> Result<(), CommandError> {
    super::install_profile_mod(&app, &*reqwest, id, r#mod, version)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn uninstall_profile_mod(id: Uuid, owner: &str, name: &str) -> Result<(), CommandError> {
    super::uninstall_profile_mod(id, owner, name)
        .await
        .map_err(Into::into)
}
