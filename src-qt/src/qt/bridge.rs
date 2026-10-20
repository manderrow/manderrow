//! The `Backend` QML singleton, through which the frontend talks to the Rust backend.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = ""]
    unsafe extern "C++" {
        include!("manderrow/cpp/native.h");

        fn manderrow_register_resources(data: &[u8]) -> bool;
        fn manderrow_configure_before_app();
        fn manderrow_enable_qml_debugging();
        fn manderrow_configure_app();
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Backend = super::BackendRust;

        /// Starts running a command. The result is delivered by [`Self::invoke_finished`] with the
        /// returned call id.
        #[qinvokable]
        fn invoke(self: Pin<&mut Backend>, command: &QString, args: &QString) -> i32;

        /// Returns the JSON translation dictionary for a locale, or an empty string if there is
        /// no such locale.
        #[qinvokable]
        fn locale_dictionary(self: &Backend, locale: &QString) -> QString;

        /// Converts GitHub-flavoured Markdown into a JSON array of blocks to lay out.
        #[qinvokable]
        fn markdown_blocks(self: &Backend, markdown: &QString) -> QString;

        /// Returns the saved window state as JSON, or an empty string if there is none.
        #[qinvokable]
        fn window_state(self: &Backend) -> QString;

        #[qinvokable]
        fn save_window_state(self: &Backend, state: &QString);

        /// Starts a new instance of the app. Returns an error message on failure.
        #[qinvokable]
        fn relaunch(self: &Backend) -> QString;

        /// Returns the value of the `MANDERROW_DEV_<NAME>` environment variable, or an empty
        /// string. These options support development and automated testing of the UI, and are
        /// only available in debug builds.
        #[qinvokable]
        fn dev_option(self: &Backend, name: &QString) -> QString;

        #[qsignal]
        fn invoke_finished(self: Pin<&mut Backend>, call_id: i32, ok: bool, payload: QString);

        /// An event emitted by the backend, with a JSON payload.
        #[qsignal]
        fn backend_event(self: Pin<&mut Backend>, name: QString, payload: QString);
    }

    impl cxx_qt::Threading for Backend {}
    impl cxx_qt::Initialize for Backend {}
}

use core::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::CommandError;

pub use qobject::{
    manderrow_configure_app, manderrow_configure_before_app, manderrow_enable_qml_debugging,
    manderrow_register_resources,
};

#[derive(Default)]
pub struct BackendRust {
    next_call_id: i32,
}

impl cxx_qt::Initialize for qobject::Backend {
    fn initialize(self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        super::app().set_event_sink(Box::new(move |name, payload| {
            let name = QString::from(name);
            // fails only once the Backend has been destroyed, when there is nothing left to notify
            _ = thread.queue(move |backend| backend.backend_event(name, QString::from(&payload)));
        }));
    }
}

impl qobject::Backend {
    fn invoke(mut self: Pin<&mut Self>, command: &QString, args: &QString) -> i32 {
        let call_id = self.next_call_id;
        self.as_mut().rust_mut().next_call_id = call_id.wrapping_add(1);

        let thread = self.qt_thread();
        let command = command.to_string();
        let args = args.to_string();
        super::runtime().spawn(async move {
            let app = super::app();
            let result = match serde_json::from_str::<serde_json::Value>(&args) {
                Ok(args) => crate::commands::dispatch(app, &command, &args).await,
                Err(e) => Err(anyhow::anyhow!("Malformed arguments: {e}").into()),
            };
            let (ok, payload) = match result {
                Ok(json) => (true, json),
                Err(e) => {
                    if let CommandError::Error { messages, .. } = &e {
                        slog_scope::error!("Command {command:?} failed: {messages:?}");
                    }
                    (false, serde_json::to_string(&e).unwrap())
                }
            };
            _ = thread.queue(move |backend| {
                backend.invoke_finished(call_id, ok, QString::from(&payload))
            });
        });
        call_id
    }

    fn locale_dictionary(&self, locale: &QString) -> QString {
        QString::from(crate::i18n::locale_dictionary(&locale.to_string()).unwrap_or_default())
    }

    fn markdown_blocks(&self, markdown: &QString) -> QString {
        let blocks = crate::markdown::to_blocks(&markdown.to_string());
        QString::from(&serde_json::to_string(&blocks).unwrap())
    }

    fn window_state(&self) -> QString {
        match crate::window_state::load() {
            Some(state) => QString::from(&serde_json::to_string(&state).unwrap()),
            None => QString::default(),
        }
    }

    fn save_window_state(&self, state: &QString) {
        let result = serde_json::from_str(&state.to_string())
            .map_err(anyhow::Error::from)
            .and_then(crate::window_state::save);
        if let Err(e) = result {
            slog_scope::error!("Unable to save window state: {e}");
        }
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
        match crate::app_commands::relaunch() {
            Ok(()) => QString::default(),
            Err(e) => QString::from(&format!("{e:#}")),
        }
    }
}
