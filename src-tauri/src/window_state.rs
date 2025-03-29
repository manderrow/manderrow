// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Save window positions and sizes and restore them when the app is reopened.

#![cfg(not(any(target_os = "android", target_os = "ios")))]

use anyhow::{anyhow, Context};
use slog_scope::error;
use tauri::{
    plugin::{Builder as PluginBuilder, TauriPlugin},
    Manager, Monitor, PhysicalPosition, PhysicalSize, RunEvent, Runtime, Window, WindowEvent,
};

use std::path::PathBuf;
use std::sync::OnceLock;
use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    sync::{Arc, Mutex},
};

use crate::util::IoErrorKindExt;

static PATH: OnceLock<PathBuf> = OnceLock::new();

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug, PartialEq, bincode::Decode, bincode::Encode)]
struct WindowState {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    // prev_x and prev_y are used to store position
    // before maximization happened, because maximization
    // will set x and y to the top-left corner of the monitor
    prev_x: i32,
    prev_y: i32,
    maximized: bool,
    fullscreen: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
            x: Default::default(),
            y: Default::default(),
            prev_x: Default::default(),
            prev_y: Default::default(),
            maximized: Default::default(),
            fullscreen: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Decode, bincode::Encode)]
enum PersistentWindowId {
    Main,
}

impl PersistentWindowId {
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "main" => Some(Self::Main),
            _ => None,
        }
    }

    pub fn as_label(self) -> &'static str {
        match self {
            PersistentWindowId::Main => "main",
        }
    }
}

type WindowStateCacheInner = HashMap<PersistentWindowId, WindowState>;
#[derive(Clone)]
struct WindowStateCache(Arc<Mutex<WindowStateCacheInner>>);
/// Used to prevent deadlocks from resize and position event listeners setting the cached state on restoring states
struct RestoringWindowState(Mutex<()>);

trait AppHandleExt {
    /// Saves all open windows state to disk
    fn save_window_state(&self) -> anyhow::Result<()>;
}

impl<R: Runtime> AppHandleExt for tauri::AppHandle<R> {
    fn save_window_state(&self) -> anyhow::Result<()> {
        let windows = self.webview_windows();
        let cache = self.state::<WindowStateCache>();
        let mut state = cache.0.lock().map_err(|e| anyhow!("{e}"))?;

        for (id, s) in state.iter_mut() {
            if let Some(window) = windows.get(id.as_label()) {
                window.as_ref().window().update_state(s)?;
            }
        }

        let path = PATH.get().context("PATH is not initialized")?;
        create_dir_all(path.parent().context("PATH initialization is broken")?)?;
        bincode::encode_into_std_write(
            &*state,
            &mut std::io::BufWriter::new(File::create(path)?),
            BINCODE_CONFIG,
        )?;

        slog_scope::debug!("Saved window state: {state:?}");

        Ok(())
    }
}

const RESTORE_SIZE: bool = true;
const RESTORE_POSITION: bool = true;
const RESTORE_MAXIMIZED: bool = true;
const RESTORE_FULLSCREEN: bool = true;

pub trait WindowExt {
    /// Restores this window state from the stored state.
    fn restore_state(&self) -> tauri::Result<()>;
}

trait PrivateWindowExt {
    fn update_state(&self, state: &mut WindowState) -> tauri::Result<()>;
}

impl<R: Runtime> WindowExt for Window<R> {
    fn restore_state(&self) -> tauri::Result<()> {
        let Some(id) = PersistentWindowId::from_label(self.label()) else {
            return Ok(());
        };

        let cache = self.state::<WindowStateCache>();

        // insert a default state if this window should be tracked and
        // the disk cache doesn't have a state for it
        let mut c = cache.0.lock().map_err(|e| anyhow!("{e}"))?;

        let restoring_window_state = self.state::<RestoringWindowState>();
        let _restoring_window_lock = restoring_window_state
            .0
            .lock()
            .map_err(|e| anyhow!("{e}"))?;

        match c.entry(id) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let state = entry.get();

                if RESTORE_SIZE {
                    self.set_size(PhysicalSize {
                        width: state.width,
                        height: state.height,
                    })?;
                }

                if RESTORE_POSITION {
                    let position = (state.x, state.y).into();
                    let size = (state.width, state.height).into();
                    // restore position to saved value if saved monitor exists
                    // otherwise, let the OS decide where to place the window
                    for m in self.available_monitors()? {
                        if m.intersects(position, size) {
                            self.set_position(PhysicalPosition {
                                x: if state.maximized {
                                    state.prev_x
                                } else {
                                    state.x
                                },
                                y: if state.maximized {
                                    state.prev_y
                                } else {
                                    state.y
                                },
                            })?;
                        }
                    }
                }

                if RESTORE_MAXIMIZED && state.maximized {
                    self.maximize()?;
                }

                if RESTORE_FULLSCREEN {
                    self.set_fullscreen(state.fullscreen)?;
                }

                slog_scope::debug!("Restored window state: {state:?}");

                Ok(())
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let state = entry.insert(WindowState::default());

                self.update_state(state)
            }
        }
    }
}

impl<R: Runtime> PrivateWindowExt for Window<R> {
    fn update_state(&self, state: &mut WindowState) -> tauri::Result<()> {
        let is_maximized =
            (RESTORE_MAXIMIZED || RESTORE_POSITION || RESTORE_SIZE) && self.is_maximized()?;
        let is_minimized = (RESTORE_POSITION || RESTORE_SIZE) && self.is_minimized()?;

        if RESTORE_MAXIMIZED {
            state.maximized = is_maximized;
        }

        if RESTORE_FULLSCREEN {
            state.fullscreen = self.is_fullscreen()?;
        }

        if RESTORE_SIZE && !is_maximized && !is_minimized {
            let size = self.inner_size()?;
            // It doesn't make sense to save a window with 0 height or width
            if size.width > 0 && size.height > 0 {
                state.width = size.width;
                state.height = size.height;
            }
        }

        if RESTORE_POSITION && !is_maximized && !is_minimized {
            let position = self.outer_position()?;
            state.x = position.x;
            state.y = position.y;
        }

        Ok(())
    }
}

fn read_window_state() -> anyhow::Result<Option<HashMap<PersistentWindowId, WindowState>>> {
    let file = match std::fs::File::open(PATH.get().context("PATH is not initialized")?) {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(Some(bincode::decode_from_reader(
        std::io::BufReader::new(file),
        BINCODE_CONFIG,
    )?))
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("window-state")
        .setup(|app, _api| {
            PATH.set(app.path().local_data_dir()?.join("window-state.bin"))
                .map_err(|_| anyhow!("Already set"))?;

            let cache = match read_window_state() {
                Ok(Some(t)) => t,
                Ok(None) => Default::default(),
                Err(e) => {
                    error!("Unable to read window state: {e}");
                    Default::default()
                }
            };
            app.manage(WindowStateCache(Arc::new(Mutex::new(cache))));
            app.manage(RestoringWindowState(Mutex::new(())));
            Ok(())
        })
        .on_window_ready(move |window| {
            let Some(id) = PersistentWindowId::from_label(window.label()) else {
                return;
            };

            if matches!(id, PersistentWindowId::Main) {
                return;
            }

            let _ = window.restore_state();

            let cache = (*window.state::<WindowStateCache>()).clone();
            let window_clone = window.clone();

            window.on_window_event(move |e| match e {
                WindowEvent::CloseRequested { .. } => {
                    let mut c = cache.0.lock().unwrap();
                    if let Some(state) = c.get_mut(&id) {
                        let _ = window_clone.update_state(state);
                    }
                }

                WindowEvent::Moved(position) if RESTORE_POSITION => {
                    if window_clone
                        .state::<RestoringWindowState>()
                        .0
                        .try_lock()
                        .is_ok()
                        && !window_clone.is_minimized().unwrap_or_default()
                    {
                        let mut c = cache.0.lock().unwrap();
                        if let Some(state) = c.get_mut(&id) {
                            state.prev_x = state.x;
                            state.prev_y = state.y;

                            state.x = position.x;
                            state.y = position.y;
                        }
                    }
                }
                WindowEvent::Resized(size) if RESTORE_SIZE => {
                    if window_clone
                        .state::<RestoringWindowState>()
                        .0
                        .try_lock()
                        .is_ok()
                    {
                        // TODO: Remove once https://github.com/tauri-apps/tauri/issues/5812 is resolved.
                        let is_maximized = if cfg!(target_os = "macos")
                            && (!window_clone.is_decorated().unwrap_or_default()
                                || !window_clone.is_resizable().unwrap_or_default())
                        {
                            false
                        } else if cfg!(target_os = "linux") {
                            // is_maximized always reports true, at least under Hyprland
                            false
                        } else {
                            window_clone.is_maximized().unwrap_or_default()
                        };

                        let is_minimized = window_clone.is_minimized().unwrap_or_default();

                        if !is_minimized && !is_maximized {
                            let mut c = cache.0.lock().unwrap();
                            if let Some(state) = c.get_mut(&id) {
                                state.width = size.width;
                                state.height = size.height;
                            }
                        }
                    }
                }
                _ => {}
            });
        })
        .on_event(move |app, event| {
            if let RunEvent::Exit = event {
                let _ = app.save_window_state();
            }
        })
        .build()
}

trait MonitorExt {
    fn intersects(&self, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> bool;
}

impl MonitorExt for Monitor {
    fn intersects(&self, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> bool {
        let PhysicalPosition { x, y } = *self.position();
        let PhysicalSize { width, height } = *self.size();

        let left = x;
        let right = x + width as i32;
        let top = y;
        let bottom = y + height as i32;

        [
            (position.x, position.y),
            (position.x + size.width as i32, position.y),
            (position.x, position.y + size.height as i32),
            (
                position.x + size.width as i32,
                position.y + size.height as i32,
            ),
        ]
        .into_iter()
        .any(|(x, y)| x >= left && x < right && y >= top && y < bottom)
    }
}
