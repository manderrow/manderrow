use tauri::{AppHandle, Emitter};

use crate::CommandError;

use super::{Settings, SettingsState, EVENT};

#[tauri::command]
pub async fn get_settings(settings: SettingsState<'_>) -> Result<Settings, CommandError> {
    settings.read().await.clone()
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    settings: SettingsState<'_>,
    updated: Settings,
) -> Result<(), CommandError> {
    let mut settings = settings.write().await;
    {
        let settings = settings.as_mut().map_err(|e| e.clone())?;
        *settings = updated;
    }
    let settings = settings.downgrade();
    let settings = settings.as_ref().unwrap();
    app.emit(EVENT, settings).map_err(anyhow::Error::from)?;
    super::write(settings)?;
    Ok(())
}
