//! Manderrow app settings. The backend is the "source of truth". When changes are made in the
//! frontend, they're sent to the backend with [`change`], which performs final validation, makes
//! the change active, sends the settings to the frontend, and finally writes them to disk.

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
    pub value: T,
    pub is_default: bool,
}

impl<T> Setting<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Setting<U> {
        Setting {
            value: f(self.value),
            is_default: self.is_default,
        }
    }
}

/// A change to a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change<T> {
    /// Changes it back to its default.
    Reset,
    Set(T),
}

impl<T> Change<T> {
    pub fn try_map<U>(self, f: impl FnOnce(T) -> Option<U>) -> Option<Change<U>> {
        Some(match self {
            Self::Reset => Change::Reset,
            Self::Set(value) => Change::Set(f(value)?),
        })
    }

    /// The value the setting is changed to, or `None` for its default.
    fn into_override(self) -> Option<T> {
        match self {
            Self::Reset => None,
            Self::Set(value) => Some(value),
        }
    }
}

/// A section of the settings, which the settings page has a tab for.
pub struct Section {
    pub id: &'static str,
    pub keys: &'static [Key],
}

/// What the settings page sets a setting with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Toggle,
    /// One of the games, or none.
    GameSelect,
    /// One of its options, by name, which is that of its translation.
    Select(&'static [&'static str]),
    Percent,
}

/// A setting's value, as the settings page shows and sets it, of the kind that its [`Input`] sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Game(Option<GameId>),
    /// One of a select's options.
    Choice(&'static str),
    Percent(Percent),
}

/// A type of setting, which decides what the settings page sets it with.
pub trait SettingType: Sized {
    const INPUT: Input;

    fn to_value(&self) -> Value;

    /// The setting of a value, or `None` if it isn't of the kind that [`Self::INPUT`] sets.
    fn from_value(value: Value) -> Option<Self>;
}

impl SettingType for bool {
    const INPUT: Input = Input::Toggle;

    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }

    fn from_value(value: Value) -> Option<Self> {
        match value {
            Value::Bool(value) => Some(value),
            _ => None,
        }
    }
}

impl SettingType for Percent {
    const INPUT: Input = Input::Percent;

    fn to_value(&self) -> Value {
        Value::Percent(*self)
    }

    fn from_value(value: Value) -> Option<Self> {
        match value {
            Value::Percent(value) => Some(value),
            _ => None,
        }
    }
}

impl SettingType for Option<GameId> {
    const INPUT: Input = Input::GameSelect;

    fn to_value(&self) -> Value {
        Value::Game(self.clone())
    }

    fn from_value(value: Value) -> Option<Self> {
        match value {
            Value::Game(value) => Some(value),
            _ => None,
        }
    }
}

impl SettingType for Theme {
    const INPUT: Input = Input::Select(&["auto", "dark", "light"]);

    fn to_value(&self) -> Value {
        Value::Choice(match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
        })
    }

    fn from_value(value: Value) -> Option<Self> {
        match value {
            Value::Choice("auto") => Some(Self::Auto),
            Value::Choice("dark") => Some(Self::Dark),
            Value::Choice("light") => Some(Self::Light),
            _ => None,
        }
    }
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

/// A game, by its id.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct GameId(pub String);

/// A percentage, from 0 to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct Percent(u8);

impl Percent {
    pub const fn new(percent: u8) -> Option<Self> {
        if percent <= 100 {
            Some(Self(percent))
        } else {
            None
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

impl<'de> serde::Deserialize<'de> for Percent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // a value out of range, as from editing the file, isn't worth losing the settings over
        Ok(Self(u8::deserialize(deserializer)?.min(100)))
    }
}

#[manderrow_macros::settings(sections = [general, appearance, launching])]
struct Settings {
    /// The game the app opens to, rather than to the game selection.
    #[section(general)]
    #[default(None)]
    default_game: Option<GameId>,

    /// Overrides the system's color scheme.
    #[section(appearance)]
    #[default(Theme::Auto)]
    theme: Theme,

    /// Shows the desktop through the window's background.
    #[section(appearance)]
    #[default(true)]
    window_translucent: bool,

    /// How opaque the window's background is while it is translucent.
    #[section(appearance)]
    #[default(Percent::new(70).unwrap())]
    window_opacity: Percent,

    /// Blurs the desktop behind the translucent window, where the platform supports it.
    #[section(appearance)]
    #[default(true)]
    window_blur: bool,

    /// Opens the console when a game is launched.
    #[section(launching)]
    #[default(false)]
    open_console_on_launch: bool,
}

/// A representation of settings that must retain complete backwards compatibility. Any necessary
/// migrations will be performed on load into [`Settings`].
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SettingsOnDisk {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_game: Option<Option<GameId>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    open_console_on_launch: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_translucent: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_opacity: Option<Percent>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_blur: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    theme: Option<Theme>,
}

/// Changes a setting, and sends the settings to the frontend.
pub async fn change(app: &AppHandle, change: SettingChange) -> Result<(), CommandError> {
    let state = app.state::<SettingsStateInner>();
    let mut settings = state.write().await;
    settings.as_mut().map_err(|e| e.clone())?.apply(change);
    let settings = settings.downgrade();
    let settings = settings.as_ref().unwrap();
    app.settings_events().emit(settings.clone());
    write(settings).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_their_defaults_until_changed() {
        let mut settings = Settings::default();
        assert_eq!(*settings.window_opacity().value, Percent::new(70).unwrap());
        assert!(settings.window_opacity().is_default);

        settings.apply(SettingChange::Theme(Change::Set(Theme::Dark)));
        assert_eq!(settings.get(Key::Theme).value, Value::Choice("dark"));
        assert!(!settings.get(Key::Theme).is_default);

        settings.apply(SettingChange::Theme(Change::Reset));
        assert_eq!(*settings.theme().value, Theme::Auto);
        assert!(settings.theme().is_default);
    }

    #[test]
    fn changes_from_the_settings_page_are_of_the_settings_types() {
        let change = SettingChange::new(Key::WindowBlur, Change::Set(Value::Bool(false)));
        assert!(matches!(
            change,
            Some(SettingChange::WindowBlur(Change::Set(false)))
        ));
        assert!(SettingChange::new(Key::WindowBlur, Change::Set(Value::Choice("dark"))).is_none());
        assert!(SettingChange::new(Key::Theme, Change::Set(Value::Choice("blue"))).is_none());
    }

    #[test]
    fn reads_what_it_writes() {
        let disk = r#"{"default_game":"lethal-company","window_opacity":150,"theme":"light"}"#;
        let disk = serde_json::from_str::<SettingsOnDisk>(disk).unwrap();
        assert_eq!(
            disk.default_game,
            Some(Some(GameId("lethal-company".to_owned())))
        );
        // clamped, rather than failing to read the settings
        assert_eq!(disk.window_opacity, Percent::new(100));
        assert_eq!(
            serde_json::to_string(&disk).unwrap(),
            r#"{"default_game":"lethal-company","window_opacity":100,"theme":"light"}"#
        );
    }
}
