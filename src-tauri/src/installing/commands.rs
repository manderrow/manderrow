use anyhow::Result;

use crate::CommandError;

#[tauri::command]
pub async fn clear_cache() -> Result<(), CommandError> {
    super::clear_cache().await.map_err(Into::into)
}
