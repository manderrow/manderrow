use crate::app::{AppHandle, RawJson};
use crate::CommandError;

use super::{SettingsPatch, SettingsStateInner, EVENT};

pub async fn get_settings(settings: &SettingsStateInner) -> Result<RawJson, CommandError> {
    let settings = settings.read().await;
    let settings = settings.as_ref().map_err(Clone::clone)?.defaulted();
    Ok(RawJson(
        serde_json::to_string(&settings).map_err(anyhow::Error::from)?,
    ))
}

pub async fn get_settings_ui() -> Result<RawJson, CommandError> {
    Ok(RawJson(super::UI.to_owned()))
}

pub async fn update_settings(
    app: &AppHandle,
    settings: &SettingsStateInner,
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
