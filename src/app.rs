//! The application handle shared by the backend.
//!
//! This replaces the subset of Tauri's `AppHandle`, `Emitter`, and `Manager` APIs that the backend
//! relied on. Events are forwarded to the frontend through an [`EventSink`], which the Qt bridge
//! installs once the QML engine has created the `Backend` singleton.

use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use anyhow::Context;

use crate::ipc::IpcState;
use crate::settings::SettingsStateInner;
use crate::Reqwest;

/// Receives every event emitted by the backend, as an event name and a JSON payload.
pub type EventSink = dyn Fn(&str, String) + Send + Sync;

#[derive(Debug, thiserror::Error)]
#[error("Failed to serialize event payload: {0}")]
pub struct EmitError(#[from] serde_json::Error);

pub type Result<T, E = EmitError> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct AppHandle(Arc<Inner>);

struct Inner {
    sink: OnceLock<Box<EventSink>>,
    reqwest: Reqwest,
    settings: SettingsStateInner,
    ipc: OnceLock<IpcState>,
}

impl AppHandle {
    pub fn new(reqwest: Reqwest, settings: SettingsStateInner) -> Self {
        Self(Arc::new(Inner {
            sink: OnceLock::new(),
            reqwest,
            settings,
            ipc: OnceLock::new(),
        }))
    }

    /// Installs the sink that events are delivered to. Events emitted before a sink is installed
    /// are dropped, because there is no frontend to receive them yet.
    pub fn set_event_sink(&self, sink: Box<EventSink>) {
        if self.0.sink.set(sink).is_err() {
            slog_scope::warn!("Event sink was already installed");
        }
    }

    pub fn set_ipc_state(&self, ipc: IpcState) {
        if self.0.ipc.set(ipc).is_err() {
            panic!("IpcState was already initialized");
        }
    }

    pub fn emit<S: serde::Serialize>(&self, event: &str, payload: S) -> Result<()> {
        let payload = serde_json::to_string(&payload)?;
        match self.0.sink.get() {
            Some(sink) => sink(event, payload),
            None => slog_scope::debug!("Dropped event {event:?}: no event sink installed"),
        }
        Ok(())
    }

    pub fn state<T: ManagedState>(&self) -> State<'_, T> {
        State(T::get(self))
    }

    /// Resolves a file bundled alongside the application, such as the injection agent.
    pub fn resource_path(&self, name: &str) -> anyhow::Result<PathBuf> {
        let exe = std::env::current_exe().context("Failed to locate the executable")?;
        let exe_dir = exe.parent().context("Executable has no parent directory")?;
        let mut candidates = vec![
            exe_dir.join(name),
            exe_dir.join("..").join("lib").join("manderrow").join(name),
        ];
        if cfg!(debug_assertions) {
            // Development builds run straight out of the target directory, so use the copy that
            // build.rs leaves in the agent's zig-out.
            candidates.push(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("agent")
                    .join("zig-out")
                    .join(name),
            );
        }
        candidates
            .iter()
            .find(|path| path.exists())
            .cloned()
            .with_context(|| format!("Unable to find resource {name:?} in {candidates:?}"))
    }
}

/// State owned by the [`AppHandle`] that can be retrieved with [`AppHandle::state`].
pub trait ManagedState: Sized + 'static {
    fn get(app: &AppHandle) -> &Self;
}

impl ManagedState for Reqwest {
    fn get(app: &AppHandle) -> &Self {
        &app.0.reqwest
    }
}

impl ManagedState for SettingsStateInner {
    fn get(app: &AppHandle) -> &Self {
        &app.0.settings
    }
}

impl ManagedState for IpcState {
    fn get(app: &AppHandle) -> &Self {
        app.0.ipc.get().expect("IpcState is not initialized")
    }
}

pub struct State<'a, T>(&'a T);

impl<T> Deref for State<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// A JSON value that has already been serialized, sent to the frontend as-is.
pub struct RawJson(pub String);

/// A stream of messages sent to the frontend while a command runs. The frontend allocates the
/// channel id and receives each message as a `channel` event.
#[derive(Clone)]
pub struct Channel {
    app: AppHandle,
    id: u32,
}

impl Channel {
    pub fn new(app: AppHandle, id: u32) -> Self {
        Self { app, id }
    }

    /// Sends a message that has already been serialized to JSON.
    pub fn send(&self, json: String) -> Result<()> {
        #[derive(serde::Serialize)]
        struct Message<'a> {
            id: u32,
            payload: &'a serde_json::value::RawValue,
        }
        let payload = serde_json::value::RawValue::from_string(json)?;
        self.app.emit(
            "channel",
            Message {
                id: self.id,
                payload: &payload,
            },
        )
    }
}
