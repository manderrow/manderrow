//! The `Settings` QML singleton: the app's settings, which the backend keeps.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QObject>);
        include!("cxx-qt-lib/qlist.h");
        type QObjectList = crate::qt::QObjectList;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("manderrow/src/qt/call.cxxqt.h");
        type Call = crate::qt::call::Call;
    }

    /// The app's color scheme.
    #[qenum(Settings)]
    enum SettingsTheme {
        /// Follows the system's.
        Auto,
        Dark,
        Light,
    }

    /// What a setting is set with on the settings page.
    #[qenum(Setting)]
    enum SettingInput {
        Toggle,
        /// One of the games, by its id.
        GameSelect,
        /// One of its `options`.
        Select,
        /// A percentage, from 0 to 100.
        Percent,
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        /// Its key, in camelCase, which is that of its translation.
        #[qproperty(QString, key, READ = name, CONSTANT)]
        #[qproperty(SettingInput, input, READ = input, CONSTANT)]
        /// The values a `Select` offers.
        #[qproperty(QStringList, options, READ = options, CONSTANT)]
        #[qproperty(QVariant, value, READ, NOTIFY)]
        #[qproperty(bool, is_default, READ, NOTIFY = value_changed)]
        type Setting = super::SettingRust;

        fn name(self: &Setting) -> QString;
        fn input(self: &Setting) -> SettingInput;
        fn options(self: &Setting) -> QStringList;

        /// Changes the setting.
        #[qinvokable]
        fn set(self: &Setting, value: &QVariant) -> *mut Call;

        /// Changes the setting back to its default.
        #[qinvokable]
        fn reset(self: &Setting) -> *mut Call;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(QString, id, READ, CONSTANT)]
        #[qproperty(QObjectList, settings, READ, CONSTANT)]
        type SettingsSection = super::SettingsSectionRust;

        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// The sections of the settings page, with their `Setting`s.
        #[qproperty(QObjectList, sections, READ, CONSTANT)]
        /// The game to open the app to, by its id, or empty.
        #[qproperty(QString, default_game, READ = default_game, NOTIFY = changed)]
        #[qproperty(SettingsTheme, theme, READ = theme, NOTIFY = changed)]
        /// Whether the window's background shows the desktop through it.
        #[qproperty(bool, window_translucent, READ = window_translucent, NOTIFY = changed)]
        /// How opaque the window's background is while it's translucent, in percent.
        #[qproperty(i32, window_opacity, READ = window_opacity, NOTIFY = changed)]
        /// Whether the desktop behind the translucent window is blurred.
        #[qproperty(bool, window_blur, READ = window_blur, NOTIFY = changed)]
        #[qproperty(bool, open_console_on_launch, READ = open_console_on_launch, NOTIFY = changed)]
        /// The messages of the error that kept the settings from being read, if one did.
        #[qproperty(QStringList, error_messages, READ, CONSTANT)]
        #[qproperty(QString, backtrace, READ, CONSTANT)]
        type Settings = super::SettingsRust;

        fn default_game(self: &Settings) -> QString;
        fn theme(self: &Settings) -> SettingsTheme;
        fn window_translucent(self: &Settings) -> bool;
        fn window_opacity(self: &Settings) -> i32;
        fn window_blur(self: &Settings) -> bool;
        fn open_console_on_launch(self: &Settings) -> bool;

        #[qsignal]
        fn changed(self: Pin<&mut Settings>);

        /// Makes a game the one to open the app to.
        #[qinvokable]
        fn set_default_game(self: &Settings, game: &QString) -> *mut Call;
    }

    impl cxx_qt::Threading for Settings {}
    impl cxx_qt::Initialize for Settings {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_setting"]
        fn make_unique() -> UniquePtr<Setting>;
        #[rust_name = "new_section"]
        fn make_unique() -> UniquePtr<SettingsSection>;
    }
}

use core::pin::Pin;

use anyhow::Context as _;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList, QVariant};

use super::call::{self, Call};
use super::child::Child;
use super::error::QmlError;
use super::outbox::Outbox;
use super::QObjectList;
use crate::settings::{
    self, Change, GameId, Input, Key, Percent, SettingChange, SettingsStateInner, Theme, Value,
    SECTIONS,
};
use qobject::{SettingInput, SettingsTheme};

pub struct SettingRust {
    value: QVariant,
    is_default: bool,
    /// Which setting it is, which it's made with.
    key: Option<Key>,
}

impl Default for SettingRust {
    fn default() -> Self {
        Self {
            value: QVariant::default(),
            is_default: true,
            key: None,
        }
    }
}

#[derive(Default)]
pub struct SettingsSectionRust {
    id: QString,
    settings: QObjectList,
}

#[derive(Default)]
pub struct SettingsRust {
    sections: QObjectList,
    error_messages: QStringList,
    backtrace: QString,
    settings: settings::Settings,
    /// The setting of every section, to update them.
    objects: Vec<Child<qobject::Setting>>,
}

impl cxx_qt::Initialize for qobject::Settings {
    fn initialize(mut self: Pin<&mut Self>) {
        let mut sections = QObjectList::default();
        for section in SECTIONS {
            let mut settings = QObjectList::default();
            for &key in section.keys {
                let mut setting = qobject::new_setting();
                setting.pin_mut().rust_mut().key = Some(key);
                let setting = Child::adopt(self.as_mut(), setting);
                settings.push(setting.as_qobject());
                self.as_mut().rust_mut().objects.push(setting);
            }
            let mut object = qobject::new_section();
            {
                let mut rust = object.pin_mut().rust_mut();
                rust.id = QString::from(section.id);
                rust.settings = settings;
            }
            sections.push(Child::adopt(self.as_mut(), object).as_qobject());
        }
        self.as_mut().rust_mut().sections = sections;

        // nothing writes them before the UI exists to
        let state = super::app().state::<SettingsStateInner>();
        let settings = match state.try_read().as_deref() {
            Ok(Ok(settings)) => settings.clone(),
            Ok(Err(e)) => {
                if let Some(QmlError {
                    messages,
                    backtrace,
                }) = QmlError::of(e)
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.error_messages = messages;
                    rust.backtrace = backtrace;
                }
                settings::Settings::default()
            }
            Err(_) => {
                slog_scope::error!("The settings were being written as the UI started");
                settings::Settings::default()
            }
        };
        self.as_mut().update(settings);

        let outbox = Outbox::new(self.qt_thread());
        super::app()
            .settings_events()
            .subscribe(move |settings| outbox.send(move |this| this.update(settings)));
    }
}

impl qobject::Settings {
    fn update(mut self: Pin<&mut Self>, settings: settings::Settings) {
        let rust = self.as_mut().rust_mut().get_mut();
        for object in &mut rust.objects {
            let setting = settings.get(object.key());
            let mut object = object.get_mut();
            let mut object_rust = object.as_mut().rust_mut();
            object_rust.value = to_qvariant(&setting.value);
            object_rust.is_default = setting.is_default;
            object.value_changed();
        }
        rust.settings = settings;
        self.changed();
    }

    fn default_game(&self) -> QString {
        self.settings
            .default_game()
            .value
            .as_ref()
            .map_or_else(QString::default, |game| QString::from(&game.0))
    }

    fn theme(&self) -> SettingsTheme {
        SettingsTheme::from(*self.settings.theme().value)
    }

    fn window_translucent(&self) -> bool {
        *self.settings.window_translucent().value
    }

    fn window_opacity(&self) -> i32 {
        i32::from(self.settings.window_opacity().value.get())
    }

    fn window_blur(&self) -> bool {
        *self.settings.window_blur().value
    }

    fn open_console_on_launch(&self) -> bool {
        *self.settings.open_console_on_launch().value
    }

    fn set_default_game(&self, game: &QString) -> *mut Call {
        let game = GameId(game.to_string());
        change(SettingChange::DefaultGame(Change::Set(Some(game))))
    }
}

impl qobject::Setting {
    fn key(&self) -> Key {
        self.key.expect("a setting is made with its key")
    }

    fn name(&self) -> QString {
        QString::from(self.key().name())
    }

    fn input(&self) -> SettingInput {
        SettingInput::from(self.key().input())
    }

    fn options(&self) -> QStringList {
        match self.key().input() {
            Input::Select(options) => options.iter().copied().map(QString::from).collect(),
            _ => QStringList::default(),
        }
    }

    fn set(&self, value: &QVariant) -> *mut Call {
        let key = self.key();
        let change = from_qvariant(key.input(), value)
            .and_then(|value| SettingChange::new(key, Change::Set(value)));
        call::spawn(async move {
            let change =
                change.with_context(|| format!("Invalid value for the setting {}", key.name()))?;
            settings::change(super::app(), change).await
        })
    }

    fn reset(&self) -> *mut Call {
        change(SettingChange::reset(self.key()))
    }
}

/// Changes a setting, as [`settings::change`] does.
fn change(change: SettingChange) -> *mut Call {
    call::spawn(settings::change(super::app(), change))
}

/// A setting's value, as QML shows it.
fn to_qvariant(value: &Value) -> QVariant {
    match value {
        Value::Bool(value) => QVariant::from(value),
        Value::Game(game) => QVariant::from(
            &game
                .as_ref()
                .map_or_else(QString::default, |game| QString::from(&game.0)),
        ),
        Value::Choice(choice) => QVariant::from(&QString::from(*choice)),
        Value::Percent(percent) => QVariant::from(&i32::from(percent.get())),
    }
}

/// A value that QML sets a setting to, if it's of the kind that its input sets.
fn from_qvariant(input: Input, value: &QVariant) -> Option<Value> {
    Some(match input {
        Input::Toggle => Value::Bool(value.value()?),
        Input::GameSelect => {
            let game = value.value::<QString>()?.to_string();
            Value::Game((!game.is_empty()).then_some(GameId(game)))
        }
        Input::Select(options) => {
            let choice = value.value::<QString>()?.to_string();
            Value::Choice(options.iter().copied().find(|option| *option == choice)?)
        }
        Input::Percent => {
            let percent = u8::try_from(value.value::<i32>()?).ok()?;
            Value::Percent(Percent::new(percent)?)
        }
    })
}

impl From<Theme> for SettingsTheme {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Auto => Self::Auto,
            Theme::Dark => Self::Dark,
            Theme::Light => Self::Light,
        }
    }
}

impl From<Input> for SettingInput {
    fn from(input: Input) -> Self {
        match input {
            Input::Toggle => Self::Toggle,
            Input::GameSelect => Self::GameSelect,
            Input::Select(_) => Self::Select,
            Input::Percent => Self::Percent,
        }
    }
}
