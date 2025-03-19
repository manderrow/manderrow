//! Manderrow app settings. The backend is the "source of truth". When changes are made in the
//! frontend, the modified settings are sent to the backend via the "update_settings" command.
//! The backend performs final validation, makes the modified settings active, and finally writes
//! them to disk.

use std::path::PathBuf;

use tauri::State;
use tokio::sync::RwLock;
use triomphe::Arc;

use crate::{paths::config_dir, product_name, util::IoErrorKindExt, CommandError};

pub mod commands;

/// The name of the event used to send the settings to the frontend.
pub const EVENT: &str = "settings";

pub type SettingsStateInner = Arc<RwLock<Result<Settings, CommandError>>>;
pub type SettingsState<'a> = State<'a, SettingsStateInner>;

fn read() -> anyhow::Result<Option<Settings>> {
    let mut bytes = match std::fs::read(get_path()) {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let settings = simd_json::from_slice::<SettingsOnDisk>(&mut bytes)?;
    Ok(Some(Settings {
        open_console_on_launch: settings.open_console_on_launch,
    }))
}

fn write(settings: &Settings) -> anyhow::Result<()> {
    let file = std::fs::File::create(get_path())?;
    simd_json::to_writer(
        file,
        &SettingsOnDisk {
            open_console_on_launch: settings.open_console_on_launch,
        },
    )?;
    Ok(())
}

fn get_path() -> PathBuf {
    config_dir().join(format!("{}.json", product_name()))
}

pub fn try_read() -> SettingsStateInner {
    Arc::new(RwLock::new(match read() {
        Ok(Some(t)) => Ok(t),
        Ok(None) => Ok(Default::default()),
        Err(e) => Err(CommandError::from(e)),
    }))
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Settings {
    #[serde(rename = "openConsoleOnLaunch")]
    open_console_on_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            open_console_on_launch: false,
        }
    }
}

/// A representation of settings that must retain complete backwards compatibility. Any necessary
/// migrations will be performed on load into [`Settings`].
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SettingsOnDisk {
    open_console_on_launch: bool,
}
