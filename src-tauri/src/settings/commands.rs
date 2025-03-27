use tauri::{ipc::Response, AppHandle, Emitter};

use crate::CommandError;

use super::{SettingsPatch, SettingsState, EVENT};

#[tauri::command]
pub async fn get_settings(settings: SettingsState<'_>) -> Result<Response, CommandError> {
    let settings = settings.read().await;
    let settings = settings.as_ref().map_err(Clone::clone)?.defaulted();
    Ok(Response::new(
        serde_json::to_string(&settings).map_err(anyhow::Error::from)?,
    ))
}

#[tauri::command]
pub async fn get_settings_ui() -> Result<Response, CommandError> {
    Ok(Response::new(super::UI.to_owned()))
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    settings: SettingsState<'_>,
    patch: SettingsPatch,
) -> Result<(), CommandError> {
    let mut settings = settings.write().await;
    settings.as_mut().map_err(|e| e.clone())?.update(patch);
    let settings = settings.downgrade();
    let settings = settings.as_ref().unwrap();
    app.emit(EVENT, settings.defaulted())
        .map_err(anyhow::Error::from)?;
    super::write(settings).await?;
    Ok(())
}
