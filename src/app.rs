//! The application handle shared by the backend.
//!
//! This replaces the subset of Tauri's `AppHandle`, `Emitter`, and `Manager` APIs that the backend
//! relied on. Its events are delivered to the parts of the frontend that show them, which
//! subscribe to them as the QML engine creates them.

use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use anyhow::Context;

use crate::ipc::IpcState;
use crate::settings::SettingsStateInner;
use crate::Reqwest;

#[derive(Clone)]
pub struct AppHandle(Arc<Inner>);

struct Inner {
    reqwest: Reqwest,
    settings: SettingsStateInner,
    ipc: OnceLock<IpcState>,
    task_events: Events<crate::tasks::Event>,
    settings_events: Events<crate::settings::Settings>,
    ipc_events: Events<crate::ipc::Event>,
    activation_events: Events<()>,
}

/// Delivers a kind of event from the backend to the part of the frontend that shows it, which
/// subscribes to it as it's created. Events emitted before then are dropped, because there is
/// nothing to show them yet.
pub struct Events<E>(OnceLock<Box<dyn Fn(E) + Send + Sync>>);

impl<E> Events<E> {
    fn new() -> Self {
        Self(OnceLock::new())
    }

    pub fn subscribe(&self, sink: impl Fn(E) + Send + Sync + 'static) {
        if self.0.set(Box::new(sink)).is_err() {
            slog_scope::warn!("Events of type {} already have a subscriber", std::any::type_name::<E>());
        }
    }

    pub fn emit(&self, event: E) {
        if let Some(sink) = self.0.get() {
            sink(event);
        }
    }
}

impl AppHandle {
    pub fn new(reqwest: Reqwest, settings: SettingsStateInner) -> Self {
        Self(Arc::new(Inner {
            reqwest,
            settings,
            ipc: OnceLock::new(),
            task_events: Events::new(),
            settings_events: Events::new(),
            ipc_events: Events::new(),
            activation_events: Events::new(),
        }))
    }

    pub fn task_events(&self) -> &Events<crate::tasks::Event> {
        &self.0.task_events
    }

    /// The settings, whenever they change.
    pub fn settings_events(&self) -> &Events<crate::settings::Settings> {
        &self.0.settings_events
    }

    pub fn ipc_events(&self) -> &Events<crate::ipc::Event> {
        &self.0.ipc_events
    }

    /// Another launch of the app asking this one to come to the front.
    pub fn activation_events(&self) -> &Events<()> {
        &self.0.activation_events
    }

    pub fn set_ipc_state(&self, ipc: IpcState) {
        if self.0.ipc.set(ipc).is_err() {
            panic!("IpcState was already initialized");
        }
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

/// Starts a new instance of the app that waits for this one to exit. The caller is responsible for
/// quitting this instance afterwards.
pub fn relaunch() -> anyhow::Result<()> {
    let exe = crate::util::launcher_path().context("Failed to locate the executable")?;
    std::process::Command::new(exe)
        .arg("--relaunch")
        .arg(std::process::id().to_string())
        .spawn()
        .context("Failed to start a new instance")?;
    Ok(())
}
