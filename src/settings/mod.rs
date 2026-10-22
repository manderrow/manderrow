//! Manderrow app settings. The backend is the "source of truth". When changes are made in the
//! frontend, the modified settings are sent to the backend via the "update_settings" command.
//! The backend performs final validation, makes the modified settings active, and finally writes
//! them to disk.

use std::path::PathBuf;
use std::sync::LazyLock;

use manderrow_paths::{config_dir, PRODUCT_NAME};
use tokio::sync::RwLock;
use triomphe::Arc;

use crate::{util::IoErrorKindExt, CommandError};

pub mod commands;

/// The name of the event used to send the settings to the frontend.
pub const EVENT: &str = "settings";

pub type SettingsStateInner = Arc<RwLock<Result<Settings, CommandError>>>;

fn read() -> anyhow::Result<Option<Settings>> {
    let mut bytes = match std::fs::read(get_path()) {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let SettingsOnDisk {
        default_game,
        open_console_on_launch,
        window_translucent,
        window_opacity,
        window_blur,
    } = simd_json::from_slice::<SettingsOnDisk>(&mut bytes)?;
    Ok(Some(Settings {
        default_game,
        open_console_on_launch,
        window_translucent,
        window_opacity,
        window_blur,
    }))
}

async fn write(
    &Settings {
        ref default_game,
        open_console_on_launch,
        window_translucent,
        window_opacity,
        window_blur,
    }: &Settings,
) -> anyhow::Result<()> {
    let settings = SettingsOnDisk {
        default_game: default_game.clone(),
        open_console_on_launch,
        window_translucent,
        window_opacity,
        window_blur,
    };
    tokio::task::spawn_blocking(move || {
        let path = get_path();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let file = std::fs::File::create(path)?;
        simd_json::to_writer(file, &settings)?;
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    Ok(())
}

static PATH: LazyLock<PathBuf> =
    LazyLock::new(|| config_dir().join(format!("{}.json", PRODUCT_NAME)));

fn get_path() -> &'static PathBuf {
    &*PATH
}

pub fn try_read() -> SettingsStateInner {
    Arc::new(RwLock::new(match read() {
        Ok(Some(t)) => Ok(t),
        Ok(None) => Ok(Default::default()),
        Err(e) => Err(CommandError::from(e)),
    }))
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Setting<T> {
    value: T,
    #[serde(rename = "isDefault")]
    is_default: bool,
}

impl<T: ToOwned> Setting<T> {
    fn to_owned(&self) -> Setting<T::Owned> {
        Setting {
            value: self.value.to_owned(),
            ..*self
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
enum Change<T> {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "override")]
    Override(T),
}

#[manderrow_macros::settings(sections = [general, appearance, launching])]
struct Settings {
    #[section(general)]
    #[default(None)]
    #[input(game_select)]
    #[ref_by(Option<&'a String>, Option::as_ref)]
    default_game: Option<String>,

    /// Shows the desktop through the window's background.
    #[section(appearance)]
    #[default(true)]
    #[input(toggle)]
    #[ref_by(bool, bool::clone)]
    window_translucent: bool,

    /// How opaque the window's background is while it is translucent, in percent.
    #[section(appearance)]
    #[default(70)]
    #[input(percent)]
    #[ref_by(u8, u8::clone)]
    window_opacity: u8,

    /// Blurs the desktop behind the translucent window, where the platform supports it.
    #[section(appearance)]
    #[default(true)]
    #[input(toggle)]
    #[ref_by(bool, bool::clone)]
    window_blur: bool,

    #[section(launching)]
    #[default(false)]
    #[input(toggle)]
    #[ref_by(bool, bool::clone)]
    open_console_on_launch: bool,
}

/// A representation of settings that must retain complete backwards compatibility. Any necessary
/// migrations will be performed on load into [`Settings`].
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SettingsOnDisk {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_game: Option<Option<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    open_console_on_launch: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_translucent: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_opacity: Option<u8>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_blur: Option<bool>,
}
