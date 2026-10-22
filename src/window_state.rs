//! Save window positions and sizes and restore them when the app is reopened.
//!
//! The frontend tracks the window geometry and hands it to [`save`] on exit. The on-disk format is
//! unchanged from the Tauri window-state plugin this replaced, so existing state is preserved.

use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::path::PathBuf;

use anyhow::Context;

use crate::util::IoErrorKindExt;

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

const FILE_NAME: &str = "window-state.bin";

#[derive(
    Debug, Default, PartialEq, bincode::Decode, bincode::Encode, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    // prev_x and prev_y are used to store position
    // before maximization happened, because maximization
    // will set x and y to the top-left corner of the monitor
    pub prev_x: i32,
    pub prev_y: i32,
    pub maximized: bool,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Decode, bincode::Encode)]
enum PersistentWindowId {
    Main,
}

type WindowStates = HashMap<PersistentWindowId, WindowState>;

fn path() -> PathBuf {
    manderrow_paths::local_data_dir().join(FILE_NAME)
}

/// Where the Tauri plugin stored the state: directly in the OS local data directory.
fn legacy_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join(FILE_NAME))
}

fn read_from(path: &PathBuf) -> anyhow::Result<Option<WindowStates>> {
    let file = match File::open(path) {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(Some(bincode::decode_from_reader(
        std::io::BufReader::new(file),
        BINCODE_CONFIG,
    )?))
}

fn read() -> anyhow::Result<Option<WindowStates>> {
    if let Some(states) = read_from(&path())? {
        return Ok(Some(states));
    }
    match legacy_path() {
        Some(path) => read_from(&path),
        None => Ok(None),
    }
}

/// Returns the saved state of the main window, if there is one.
pub fn load() -> Option<WindowState> {
    match read() {
        Ok(states) => {
            let state = states?.remove(&PersistentWindowId::Main)?;
            slog_scope::debug!("Restored window state: {state:?}");
            Some(state)
        }
        Err(e) => {
            slog_scope::error!("Unable to read window state: {e}");
            None
        }
    }
}

pub fn save(state: WindowState) -> anyhow::Result<()> {
    let path = path();
    create_dir_all(path.parent().context("Window state path has no parent")?)?;
    slog_scope::debug!("Saved window state: {state:?}");
    let states = WindowStates::from([(PersistentWindowId::Main, state)]);
    bincode::encode_into_std_write(
        &states,
        &mut std::io::BufWriter::new(File::create(path)?),
        BINCODE_CONFIG,
    )?;
    Ok(())
}
