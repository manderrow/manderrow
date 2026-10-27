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
        #[qproperty(QString, key, READ, CONSTANT)]
        #[qproperty(SettingInput, input, READ, CONSTANT)]
        /// The values a `Select` offers.
        #[qproperty(QStringList, options, READ, CONSTANT)]
        #[qproperty(QVariant, value, READ, NOTIFY)]
        #[qproperty(bool, is_default, READ, NOTIFY = value_changed)]
        type Setting = super::SettingRust;

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
        #[qproperty(QString, default_game, READ, NOTIFY = changed)]
        #[qproperty(SettingsTheme, theme, READ, NOTIFY = changed)]
        /// Whether the window's background shows the desktop through it.
        #[qproperty(bool, window_translucent, READ, NOTIFY = changed)]
        /// How opaque the window's background is while it's translucent, in percent.
        #[qproperty(i32, window_opacity, READ, NOTIFY = changed)]
        /// Whether the desktop behind the translucent window is blurred.
        #[qproperty(bool, window_blur, READ, NOTIFY = changed)]
        #[qproperty(bool, open_console_on_launch, READ, NOTIFY = changed)]
        /// The messages of the error that kept the settings from being read, if one did.
        #[qproperty(QStringList, error_messages, READ, CONSTANT)]
        #[qproperty(QString, backtrace, READ, CONSTANT)]
        type Settings = super::SettingsRust;

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

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList, QVariant};
use serde_json::json;

use super::call::{self, Call};
use super::child::Child;
use super::outbox::Outbox;
use super::QObjectList;
use crate::settings::{self, Input, SettingsStateInner, Theme, SECTIONS};
use qobject::{SettingInput, SettingsTheme};

pub struct SettingRust {
    key: QString,
    input: SettingInput,
    options: QStringList,
    value: QVariant,
    is_default: bool,
    /// Its key, for the backend.
    name: &'static str,
}

impl Default for SettingRust {
    fn default() -> Self {
        Self {
            key: QString::default(),
            input: SettingInput::Toggle,
            options: QStringList::default(),
            value: QVariant::default(),
            is_default: true,
            name: "",
        }
    }
}

#[derive(Default)]
pub struct SettingsSectionRust {
    id: QString,
    settings: QObjectList,
}

pub struct SettingsRust {
    sections: QObjectList,
    default_game: QString,
    theme: SettingsTheme,
    window_translucent: bool,
    window_opacity: i32,
    window_blur: bool,
    open_console_on_launch: bool,
    error_messages: QStringList,
    backtrace: QString,
    /// The settings of every section, for updating them.
    settings: Vec<Child<qobject::Setting>>,
}

impl Default for SettingsRust {
    fn default() -> Self {
        Self {
            sections: QObjectList::default(),
            default_game: QString::default(),
            theme: SettingsTheme::Auto,
            window_translucent: true,
            window_opacity: 70,
            window_blur: true,
            open_console_on_launch: false,
            error_messages: QStringList::default(),
            backtrace: QString::default(),
            settings: Vec::new(),
        }
    }
}

impl cxx_qt::Initialize for qobject::Settings {
    fn initialize(mut self: Pin<&mut Self>) {
        let mut sections = QObjectList::default();
        for info in SECTIONS {
            let mut section = qobject::new_section();
            let mut settings = QObjectList::default();
            for setting_info in info.settings {
                let mut setting = qobject::new_setting();
                {
                    let mut rust = setting.pin_mut().rust_mut();
                    rust.key = QString::from(setting_info.key);
                    rust.name = setting_info.key;
                    rust.input = match setting_info.input {
                        Input::Toggle => SettingInput::Toggle,
                        Input::GameSelect => SettingInput::GameSelect,
                        Input::Select => SettingInput::Select,
                        Input::Percent => SettingInput::Percent,
                    };
                    rust.options = setting_info
                        .options
                        .iter()
                        .copied()
                        .map(QString::from)
                        .collect();
                }
                let setting = Child::adopt(self.as_mut(), setting);
                settings.0.append(setting.as_qobject());
                self.as_mut().rust_mut().settings.push(setting);
            }
            {
                let mut rust = section.pin_mut().rust_mut();
                rust.id = QString::from(info.id);
                rust.settings = settings;
            }
            sections
                .0
                .append(Child::adopt(self.as_mut(), section).as_qobject());
        }
        self.as_mut().rust_mut().sections = sections;

        // nothing writes them before the UI exists to
        let state = super::app().state::<SettingsStateInner>();
        match state.try_read().as_deref() {
            Ok(Ok(settings)) => self.as_mut().update(settings),
            Ok(Err(crate::CommandError::Error {
                messages,
                backtrace,
            })) => {
                let mut rust = self.as_mut().rust_mut();
                rust.error_messages = messages.iter().map(QString::from).collect();
                rust.backtrace = QString::from(backtrace);
            }
            Ok(Err(crate::CommandError::Aborted)) => {}
            Err(_) => slog_scope::error!("The settings were being written as the UI started"),
        }

        let outbox = Outbox::new(self.qt_thread());
        super::app()
            .settings_events()
            .subscribe(move |settings| outbox.send(move |this| this.update(&settings)));
    }
}

impl qobject::Settings {
    fn update(mut self: Pin<&mut Self>, settings: &settings::Settings) {
        {
            let mut rust = self.as_mut().rust_mut();
            rust.default_game = settings
                .default_game()
                .value()
                .map(|game| QString::from(game.as_str()))
                .unwrap_or_default();
            rust.theme = match settings.theme().value() {
                Theme::Auto => SettingsTheme::Auto,
                Theme::Dark => SettingsTheme::Dark,
                Theme::Light => SettingsTheme::Light,
            };
            rust.window_translucent = *settings.window_translucent().value();
            rust.window_opacity = i32::from(*settings.window_opacity().value());
            rust.window_blur = *settings.window_blur().value();
            rust.open_console_on_launch = *settings.open_console_on_launch().value();
            for setting in &mut rust.settings {
                let Some((value, is_default)) = settings.json(setting.get().name) else {
                    continue;
                };
                let value = match value {
                    serde_json::Value::Bool(value) => QVariant::from(&value),
                    serde_json::Value::Number(value) => match value.as_i64() {
                        Some(value) => QVariant::from(&value),
                        None => QVariant::from(&value.as_f64().unwrap_or_default()),
                    },
                    serde_json::Value::String(value) => QVariant::from(&QString::from(&value)),
                    _ => QVariant::default(),
                };
                let mut setting = setting.get_mut();
                let mut rust = setting.as_mut().rust_mut();
                rust.value = value;
                rust.is_default = is_default;
                setting.value_changed();
            }
        }
        self.changed();
    }

    fn set_default_game(&self, game: &QString) -> *mut Call {
        update(json!({ "defaultGame": { "override": game.to_string() } }))
    }
}

impl qobject::Setting {
    fn set(&self, value: &QVariant) -> *mut Call {
        let value = match self.input {
            SettingInput::Toggle => value.value::<bool>().map(serde_json::Value::from),
            SettingInput::Percent => value.value::<i32>().map(serde_json::Value::from),
            _ => value
                .value::<QString>()
                .map(|value| value.to_string().into()),
        };
        let name = self.name;
        match value {
            Some(value) => update(json!({ name: { "override": value } })),
            None => call::start(
                None,
                async move { Err(anyhow::anyhow!("Invalid value for the setting {name}").into()) },
                |_, ()| {},
            ),
        }
    }

    fn reset(&self) -> *mut Call {
        update(json!({ self.name: "default" }))
    }
}

/// Changes the settings, as [`settings::update`] does.
fn update(patch: serde_json::Value) -> *mut Call {
    call::start(None, settings::update(super::app(), patch), |_, ()| {})
}
