//! The `Backend` QML singleton, through which the frontend talks to the Rust backend.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
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

        /// Events emitted by the backend, as a JSON array of `{ name, payload }` objects in the
        /// order they were emitted. Events are batched, so that a flood of them, such as a game's
        /// log, doesn't cost the UI thread a signal for each one.
        #[qsignal]
        fn backend_events(self: Pin<&mut Backend>, events: QString);
    }

    impl cxx_qt::Threading for Backend {}
    impl cxx_qt::Initialize for Backend {}
}

use core::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::QString;
use parking_lot::Mutex;

use crate::CommandError;

#[derive(Default)]
pub struct BackendRust {
    next_call_id: i32,
    outbox: Option<Arc<Outbox>>,
}

/// Something to deliver to the frontend.
enum Delivery {
    Event {
        name: String,
        payload: String,
    },
    Finished {
        call_id: i32,
        ok: bool,
        payload: String,
    },
}

/// Delivers events and command results to the frontend in the order they were sent, with
/// everything sent before the UI thread gets to it delivered at once.
struct Outbox {
    thread: CxxQtThread<qobject::Backend>,
    queue: Mutex<Vec<Delivery>>,
}

impl Outbox {
    fn send(self: &Arc<Self>, delivery: Delivery) {
        let mut queue = self.queue.lock();
        let first = queue.is_empty();
        queue.push(delivery);
        drop(queue);
        // a delivery is already on its way otherwise, which will take this one along
        if first {
            let outbox = self.clone();
            // fails only once the Backend has been destroyed, when there is nothing left to notify
            _ = self.thread.queue(move |backend| outbox.deliver(backend));
        }
    }

    fn deliver(&self, mut backend: Pin<&mut qobject::Backend>) {
        let deliveries = std::mem::take(&mut *self.queue.lock());
        let mut events = String::new();
        let flush = |backend: Pin<&mut qobject::Backend>, events: &mut String| {
            if !events.is_empty() {
                events.push(']');
                backend.backend_events(QString::from(&*events));
                events.clear();
            }
        };
        for delivery in deliveries {
            match delivery {
                Delivery::Event { name, payload } => {
                    events.push(if events.is_empty() { '[' } else { ',' });
                    events.push_str(r#"{"name":"#);
                    events.push_str(&serde_json::to_string(&name).unwrap());
                    events.push_str(r#","payload":"#);
                    // serialized by `AppHandle::emit`
                    events.push_str(&payload);
                    events.push('}');
                }
                Delivery::Finished {
                    call_id,
                    ok,
                    payload,
                } => {
                    flush(backend.as_mut(), &mut events);
                    backend
                        .as_mut()
                        .invoke_finished(call_id, ok, QString::from(&payload));
                }
            }
        }
        flush(backend, &mut events);
    }
}

impl cxx_qt::Initialize for qobject::Backend {
    fn initialize(mut self: Pin<&mut Self>) {
        let outbox = Arc::new(Outbox {
            thread: self.qt_thread(),
            queue: Mutex::new(Vec::new()),
        });
        self.as_mut().rust_mut().outbox = Some(outbox.clone());
        super::app().set_event_sink(Box::new(move |name, payload| {
            outbox.send(Delivery::Event {
                name: name.to_owned(),
                payload,
            });
        }));
    }
}

impl qobject::Backend {
    fn invoke(mut self: Pin<&mut Self>, command: &QString, args: &QString) -> i32 {
        let call_id = self.next_call_id;
        self.as_mut().rust_mut().next_call_id = call_id.wrapping_add(1);

        let outbox = self.outbox.clone().expect("Backend is initialized");
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
            outbox.send(Delivery::Finished {
                call_id,
                ok,
                payload,
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
