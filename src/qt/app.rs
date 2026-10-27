//! The `App` QML singleton: the app's window state, translations, and the like, which aren't the
//! backend's to keep.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("manderrow/src/qt/call.cxxqt.h");
        type Call = crate::qt::call::Call;
        include!("manderrow/src/qt/markdown.cxxqt.h");
        type MarkdownDocument = crate::qt::markdown::qobject::MarkdownDocument;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        /// Whether the state was saved when the app last closed, rather than these defaults.
        #[qproperty(bool, saved, READ, CONSTANT)]
        #[qproperty(i32, width, READ, WRITE, NOTIFY)]
        #[qproperty(i32, height, READ, WRITE, NOTIFY)]
        #[qproperty(i32, x, READ, WRITE, NOTIFY)]
        #[qproperty(i32, y, READ, WRITE, NOTIFY)]
        /// The position before the last move, which is where a maximized window was, since
        /// maximizing moves it to the corner of its screen.
        #[qproperty(i32, prev_x, READ, WRITE, NOTIFY)]
        #[qproperty(i32, prev_y, READ, WRITE, NOTIFY)]
        #[qproperty(bool, maximized, READ, WRITE, NOTIFY)]
        #[qproperty(bool, fullscreen, READ, WRITE, NOTIFY)]
        type WindowState = super::WindowStateRust;

        /// Saves the state, for when the app is next opened.
        #[qinvokable]
        fn save(self: &WindowState);

        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// The main window's position and size, which it's restored to.
        #[qproperty(*mut WindowState, window_state, READ, CONSTANT)]
        type App = super::AppRust;

        /// The JSON translation dictionary of a locale, or empty if there's no such locale.
        #[qinvokable]
        fn locale_dictionary(self: &App, locale: &QString) -> QString;

        /// The user's locales, in their order of preference.
        #[qinvokable]
        fn preferred_locales(self: &App) -> QStringList;

        /// Lays out GitHub-flavoured Markdown as blocks.
        #[qinvokable]
        fn markdown(self: &App, markdown: &QString) -> *mut MarkdownDocument;

        /// Deletes the cached downloads.
        #[qinvokable]
        fn clear_cache(self: &App) -> *mut Call;

        /// Starts a new instance of the app. Returns an error message on failure.
        #[qinvokable]
        fn relaunch(self: &App) -> QString;

        /// Returns the value of the `MANDERROW_DEV_<NAME>` environment variable, or an empty
        /// string. These options support development and automated testing of the UI, and are
        /// only available in debug builds.
        #[qinvokable]
        fn dev_option(self: &App, name: &QString) -> QString;

        /// Exits once the splashscreen shows, with `EXIT_ON_SPLASH` set, for benchmarking.
        #[qinvokable]
        fn bench_exit_splash(self: &App);

        /// Exits once the app is interactive, with `EXIT_ON_INTERACTIVE` set, for benchmarking.
        #[qinvokable]
        fn bench_exit_interactive(self: &App);

        /// Another launch of the app asked this one to come to the front.
        #[qsignal]
        fn activation_requested(self: Pin<&mut App>);
    }

    impl cxx_qt::Threading for App {}
    impl cxx_qt::Initialize for App {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_window_state"]
        fn make_unique() -> UniquePtr<WindowState>;
    }
}

use core::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};

use super::call::{self, Call};
use super::child::Child;
use super::markdown::{self, qobject::MarkdownDocument};
use super::outbox::Outbox;
use crate::window_state;

pub struct WindowStateRust {
    saved: bool,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    prev_x: i32,
    prev_y: i32,
    maximized: bool,
    fullscreen: bool,
}

impl Default for WindowStateRust {
    fn default() -> Self {
        Self {
            saved: false,
            width: 1024,
            height: 576,
            x: 0,
            y: 0,
            prev_x: 0,
            prev_y: 0,
            maximized: false,
            fullscreen: false,
        }
    }
}

pub struct AppRust {
    window_state: *mut qobject::WindowState,
}

impl Default for AppRust {
    fn default() -> Self {
        Self {
            window_state: std::ptr::null_mut(),
        }
    }
}

impl cxx_qt::Initialize for qobject::App {
    fn initialize(mut self: Pin<&mut Self>) {
        let mut state = qobject::new_window_state();
        if let Some(saved) = window_state::load() {
            let mut rust = state.pin_mut().rust_mut();
            rust.saved = true;
            rust.width = i32::try_from(saved.width).unwrap_or(i32::MAX);
            rust.height = i32::try_from(saved.height).unwrap_or(i32::MAX);
            rust.x = saved.x;
            rust.y = saved.y;
            rust.prev_x = saved.prev_x;
            rust.prev_y = saved.prev_y;
            rust.maximized = saved.maximized;
            rust.fullscreen = saved.fullscreen;
        }
        let state = Child::adopt(self.as_mut(), state);
        self.as_mut().rust_mut().window_state = state.as_ptr();

        let outbox = Outbox::new(self.qt_thread());
        super::app()
            .activation_events()
            .subscribe(move |()| outbox.send(|app| app.activation_requested()));
    }
}

impl qobject::WindowState {
    fn save(&self) {
        let state = window_state::WindowState {
            width: u32::try_from(self.width).unwrap_or(0),
            height: u32::try_from(self.height).unwrap_or(0),
            x: self.x,
            y: self.y,
            prev_x: self.prev_x,
            prev_y: self.prev_y,
            maximized: self.maximized,
            fullscreen: self.fullscreen,
        };
        if let Err(e) = window_state::save(state) {
            slog_scope::error!("Unable to save window state: {e}");
        }
    }
}

impl qobject::App {
    fn locale_dictionary(&self, locale: &QString) -> QString {
        QString::from(crate::i18n::locale_dictionary(&locale.to_string()).unwrap_or_default())
    }

    fn preferred_locales(&self) -> QStringList {
        crate::i18n::get_preferred_locales()
            .iter()
            .map(QString::from)
            .collect()
    }

    fn markdown(&self, markdown: &QString) -> *mut MarkdownDocument {
        // QML owns an object without a parent that a method returns
        markdown::new_document(&markdown.to_string()).into_raw()
    }

    fn clear_cache(&self) -> *mut Call {
        call::start(
            None,
            async { Ok(crate::installing::clear_cache().await?) },
            |_, ()| {},
        )
    }

    fn dev_option(&self, name: &QString) -> QString {
        // development options are not available in release builds
        if !cfg!(debug_assertions) {
            return QString::default();
        }
        let key = format!("MANDERROW_DEV_{}", name.to_string().to_uppercase());
        QString::from(&std::env::var(key).unwrap_or_default())
    }

    fn relaunch(&self) -> QString {
        match crate::app::relaunch() {
            Ok(()) => QString::default(),
            Err(e) => QString::from(&format!("{e:#}")),
        }
    }

    fn bench_exit_splash(&self) {
        if std::env::var_os("EXIT_ON_SPLASH").is_some() {
            std::process::exit(0);
        }
    }

    fn bench_exit_interactive(&self) {
        if std::env::var_os("EXIT_ON_INTERACTIVE").is_some() {
            std::process::exit(0);
        }
    }
}
