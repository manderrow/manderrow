// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Save window positions and sizes and restore them when the app is reopened.

#![cfg(not(any(target_os = "android", target_os = "ios")))]

use log::error;
use tauri::{
    plugin::{Builder as PluginBuilder, TauriPlugin},
    Manager, Monitor, PhysicalPosition, PhysicalSize, RunEvent, Runtime, Window, WindowEvent,
};

use std::path::PathBuf;
use std::sync::LazyLock;
use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    sync::{Arc, Mutex},
};

use crate::Error;

/// Default filename used to store window state.
///
/// If using a custom filename, you should probably use [`AppHandleExt::filename`] instead.
static PATH: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::data_local_dir().unwrap().join("window-state.bin"));

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

type Result<T> = std::result::Result<T, Error>;

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

struct WindowStateCache(Arc<Mutex<HashMap<String, WindowState>>>);
/// Used to prevent deadlocks from resize and position event listeners setting the cached state on restoring states
struct RestoringWindowState(Mutex<()>);

trait AppHandleExt {
    /// Saves all open windows state to disk
    fn save_window_state(&self) -> Result<()>;
}

impl<R: Runtime> AppHandleExt for tauri::AppHandle<R> {
    fn save_window_state(&self) -> Result<()> {
        let windows = self.webview_windows();
        let cache = self.state::<WindowStateCache>();
        let mut state = cache.0.lock().unwrap();

        for (label, s) in state.iter_mut() {
            if let Some(window) = windows.get(label) {
                window.as_ref().window().update_state(s)?;
            }
        }

        create_dir_all(PATH.parent().unwrap())?;
        bincode::encode_into_std_write(
            &*state,
            &mut std::io::BufWriter::new(File::create(&*PATH)?),
            BINCODE_CONFIG,
        )?;
        Ok(())
    }
}

const RESTORE_SIZE: bool = true;
const RESTORE_POSITION: bool = true;
const RESTORE_MAXIMIZED: bool = true;
const RESTORE_FULLSCREEN: bool = true;

trait WindowExt {
    /// Restores this window state from disk
    fn restore_state(&self) -> tauri::Result<()>;

    fn update_state(&self, state: &mut WindowState) -> tauri::Result<()>;
}

impl<R: Runtime> WindowExt for Window<R> {
    fn restore_state(&self) -> tauri::Result<()> {
        let label = self.label();

        let restoring_window_state = self.state::<RestoringWindowState>();
        let _restoring_window_lock = restoring_window_state.0.lock().unwrap();
        let cache = self.state::<WindowStateCache>();
        let mut c = cache.0.lock().unwrap();

        if let Some(state) = c
            .get(label)
            .filter(|state| state != &&WindowState::default())
        {
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
        } else {
            let mut metadata = WindowState::default();

            if RESTORE_SIZE {
                let size = self.inner_size()?;
                metadata.width = size.width;
                metadata.height = size.height;
            }

            if RESTORE_POSITION {
                let pos = self.outer_position()?;
                metadata.x = pos.x;
                metadata.y = pos.y;
            }

            if RESTORE_MAXIMIZED {
                metadata.maximized = self.is_maximized()?;
            }

            if RESTORE_FULLSCREEN {
                metadata.fullscreen = self.is_fullscreen()?;
            }

            c.insert(label.into(), metadata);
        }

        Ok(())
    }

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

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("window-state")
        .setup(|app, _api| {
            let cache = std::fs::File::open(&*PATH)
                .inspect_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        error!("Unable to read window state: {e}");
                    }
                })
                .map_err(|_| ())
                .and_then(|rdr| {
                    bincode::decode_from_reader(std::io::BufReader::new(rdr), BINCODE_CONFIG)
                        .map_err(|_| ())
                })
                .unwrap_or_default();
            app.manage(WindowStateCache(Arc::new(Mutex::new(cache))));
            app.manage(RestoringWindowState(Mutex::new(())));
            Ok(())
        })
        .on_window_ready(move |window| {
            let label = window.label();

            if label == "splashscreen" {
                return;
            }

            let _ = window.restore_state();

            let cache = window.state::<WindowStateCache>();
            let cache = cache.0.clone();
            let label = label.to_string();
            let window_clone = window.clone();

            // insert a default state if this window should be tracked and
            // the disk cache doesn't have a state for it
            {
                cache
                    .lock()
                    .unwrap()
                    .entry(label.clone())
                    .or_insert_with(WindowState::default);
            }

            window.on_window_event(move |e| match e {
                WindowEvent::CloseRequested { .. } => {
                    let mut c = cache.lock().unwrap();
                    if let Some(state) = c.get_mut(&label) {
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
                        let mut c = cache.lock().unwrap();
                        if let Some(state) = c.get_mut(&label) {
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
                        } else {
                            window_clone.is_maximized().unwrap_or_default()
                        };

                        if !window_clone.is_minimized().unwrap_or_default() && !is_maximized {
                            let mut c = cache.lock().unwrap();
                            if let Some(state) = c.get_mut(&label) {
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
