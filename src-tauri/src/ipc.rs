use std::collections::HashMap;
use std::ffi::OsString;

use ipc_channel::ipc::IpcOneShotServer;
use log::{debug, error};
use tauri::Emitter;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub enum SafeOsString {
    Unicode(String),
    NonUnicodeBytes(Vec<u8>),
    NonUnicodeWide(Vec<u16>),
    NonUnicodeOther(String),
}

impl From<OsString> for SafeOsString {
    fn from(value: OsString) -> Self {
        match value.into_string() {
            Ok(s) => Self::Unicode(s),
            #[cfg(unix)]
            Err(s) => {
                use std::os::unix::ffi::OsStrExt;
                Self::NonUnicodeBytes(s.as_bytes().to_owned())
            }
            #[cfg(windows)]
            Err(s) => {
                use std::os::windows::ffi::OsStrExt;
                Self::NonUnicodeWide(s.encode_wide().collect::<Vec<_>>())
            }
            #[cfg(not(any(unix, windows)))]
            Err(s) => Self::NonUnicodeOther(format!("{s:?}")),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum StandardOutputChannel {
    Out,
    Err,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum OutputLine {
    Unicode(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum C2SMessage {
    Start {
        command: SafeOsString,
        args: Vec<SafeOsString>,
        env: HashMap<String, SafeOsString>,
    },
    Log {
        level: log::Level,
        message: String,
    },
    Output {
        channel: StandardOutputChannel,
        line: OutputLine,
    },
    Exit {
        code: Option<i32>,
    },
    Crash {
        error: String,
    },
}

pub fn spawn_server_listener<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    c2s_rx: IpcOneShotServer<C2SMessage>,
) -> anyhow::Result<()> {
    let app = app.clone();
    std::thread::Builder::new()
        .name("ipc-receiver".to_owned())
        .spawn(move || {
            let (rx, mut msg) = match c2s_rx.accept() {
                Ok(t) => t,
                Err(e) => {
                    error!("Unable to receive IPC message: {e}");
                    return;
                }
            };
            loop {
                debug!("Received message from client: {msg:?}");
                if let Err(e) = app.emit_to("main", "ipc-message", msg) {
                    error!("Unable to emit ipc-message event to webview: {e}");
                }
                msg = match rx.recv() {
                    Ok(t) => t,
                    Err(e) => {
                        error!("Unable to receive IPC message: {e}");
                        break;
                    }
                };
            }
        })?;
    Ok(())
}
