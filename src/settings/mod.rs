//! Manderrow app settings. The backend is the "source of truth". When changes are made in the
//! frontend, the modified settings are sent to the backend with [`update`], which performs final
//! validation, makes the modified settings active, sends them to the frontend, and finally writes
//! them to disk.

use std::path::PathBuf;
use std::sync::LazyLock;

use manderrow_paths::{config_dir, PRODUCT_NAME};
use tokio::sync::RwLock;
use triomphe::Arc;

use crate::app::AppHandle;
use crate::{util::IoErrorKindExt, CommandError};

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
        theme,
    } = simd_json::from_slice::<SettingsOnDisk>(&mut bytes)?;
    Ok(Some(Settings {
        default_game,
        open_console_on_launch,
        theme,
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
        theme,
    }: &Settings,
) -> anyhow::Result<()> {
    let settings = SettingsOnDisk {
        default_game: default_game.clone(),
        open_console_on_launch,
        window_translucent,
        window_opacity,
        window_blur,
        theme,
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
    &PATH
}

pub fn try_read() -> SettingsStateInner {
    Arc::new(RwLock::new(match read() {
        Ok(Some(t)) => Ok(t),
        Ok(None) => Ok(Default::default()),
        Err(e) => Err(CommandError::from(e)),
    }))
}

/// A setting's value, which is its default unless it's been changed.
#[derive(Debug, Clone, Copy)]
pub struct Setting<T> {
    value: T,
    is_default: bool,
}

impl<T> Setting<T> {
    pub fn value(&self) -> &T {
        &self.value
    }
}

/// A section of the settings, which the settings page has a tab for.
pub struct SectionInfo {
    pub id: &'static str,
    pub settings: &'static [SettingInfo],
}

/// What a setting is set with on the settings page.
pub struct SettingInfo {
    /// Its key, in camelCase, which is that of its translation.
    pub key: &'static str,
    pub input: Input,
    /// The values a `Select` offers.
    pub options: &'static [&'static str],
}

pub enum Input {
    Toggle,
    /// One of the games, by its id.
    GameSelect,
    /// One of its options.
    Select,
    /// A percentage, from 0 to 100.
    Percent,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
enum Change<T> {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "override")]
    Override(T),
}

/// The app's color scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follows the system's color scheme.
    Auto,
    Dark,
    Light,
}

#[manderrow_macros::settings(sections = [general, appearance, launching])]
struct Settings {
    #[section(general)]
    #[default(None)]
    #[input(game_select)]
    #[ref_by(Option<&'a String>, Option::as_ref)]
    default_game: Option<String>,

    /// Overrides the system's color scheme.
    #[section(appearance)]
    #[default(Theme::Auto)]
    #[input(select)]
    #[options(auto, dark, light)]
    #[ref_by(Theme, Theme::clone)]
    theme: Theme,

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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    theme: Option<Theme>,
}

/// Changes the settings: `patch` is a JSON object of the changes by key, each `"default"`, or
/// `{ "override": value }`.
pub async fn update(app: &AppHandle, patch: serde_json::Value) -> Result<(), CommandError> {
    let patch: SettingsPatch = serde_json::from_value(patch).map_err(anyhow::Error::from)?;
    let state = app.state::<SettingsStateInner>();
    let mut settings = state.write().await;
    settings.as_mut().map_err(|e| e.clone())?.update(patch);
    let settings = settings.downgrade();
    let settings = settings.as_ref().unwrap();
    app.settings_events().emit(settings.clone());
    write(settings).await?;
    Ok(())
}
