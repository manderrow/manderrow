//! Client is the game, server is the Manderrow app.

use std::collections::HashMap;
use std::ffi::OsString;

use anyhow::{bail, Context as _, Result};
use ipc_channel::ipc::{IpcError, IpcOneShotServer, IpcReceiver, IpcSender};
use slog::error;
use smol_str::SmolStr;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use triomphe::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub enum SafeOsString {
    Unicode(String),
    NonUnicodeBytes(Vec<u8>),
    NonUnicodeWide(Vec<u16>),
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
            Err(s) => compile_error!("Unsupported platform"),
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

impl OutputLine {
    pub fn new(bytes: Vec<u8>) -> Self {
        String::from_utf8(bytes)
            .map(|s| OutputLine::Unicode(s))
            .unwrap_or_else(|e| OutputLine::Bytes(e.into_bytes()))
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DoctorFix<T> {
    pub id: T,
    pub label: Option<HashMap<String, serde_json::Value>>,
    pub confirm_label: Option<HashMap<String, serde_json::Value>>,
    pub description: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DoctorReport {
    pub id: Uuid,
    pub translation_key: String,
    pub message: Option<String>,
    pub message_args: Option<HashMap<String, serde_json::Value>>,
    pub fixes: Vec<DoctorFix<String>>,
}

#[derive(Debug, Copy, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogLevel {
    Critical,
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

impl From<slog::Level> for LogLevel {
    fn from(value: slog::Level) -> Self {
        match value {
            slog::Level::Critical => Self::Critical,
            slog::Level::Error => Self::Error,
            slog::Level::Warning => Self::Warning,
            slog::Level::Info => Self::Info,
            slog::Level::Debug => Self::Debug,
            slog::Level::Trace => Self::Trace,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum C2SMessage {
    Connect {
        s2c_tx: String,
    },
    Disconnect {},
    Start {
        command: SafeOsString,
        args: Vec<SafeOsString>,
        env: HashMap<String, SafeOsString>,
    },
    Log {
        level: LogLevel,
        scope: SmolStr,
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
    DoctorReport(DoctorReport),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum S2CMessage {
    Connect,
    PatientResponse { id: Uuid, choice: String },
    Kill,
}

pub struct IpcState {
    pub s2c_tx: flume::Sender<S2CMessage>,
    pub s2c_rx: flume::Receiver<S2CMessage>,
}

impl Default for IpcState {
    fn default() -> Self {
        let (s2c_tx, s2c_rx) = flume::bounded(1);
        Self { s2c_tx, s2c_rx }
    }
}

impl IpcState {
    pub fn bi<'a>(&'a self, channel: &'a Channel<C2SMessage>) -> IpcBiState<'a> {
        IpcBiState {
            ipc_state: self,
            c2s_tx: channel,
        }
    }
}

pub fn spawn_c2s_pipe(
    log: slog::Logger,
    app_handle: AppHandle,
    c2s_channel: Channel<C2SMessage>,
    c2s_rx: IpcOneShotServer<C2SMessage>,
) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .name("ipc-receiver".to_owned())
        .spawn(move || {
            let (rx, mut msg) = match c2s_rx.accept() {
                Ok(t) => t,
                Err(e) => {
                    error!(log, "Unable to receive IPC message: {e}");
                    return;
                }
            };
            let mut exited = false;
            loop {
                match msg {
                    C2SMessage::Connect { ref mut s2c_tx } => {
                        if let Err(e) =
                            spawn_s2c_pipe(log.clone(), &app_handle, std::mem::take(s2c_tx))
                        {
                            error!(log, "Failed to spawn S2C IPC pipe: {e}");
                        }
                    }
                    C2SMessage::Crash { .. } | C2SMessage::Exit { .. } => {
                        exited = true;
                    }
                    _ => {}
                }
                if let Err(e) = c2s_channel.send(msg) {
                    // log this to the global logger because if we can't send messages on the channel, the local logger will fail
                    error!(
                        slog_scope::logger(),
                        "Unable to emit ipc-message event to webview: {e}"
                    );
                }
                msg = match rx.recv() {
                    Ok(t) => t,
                    Err(IpcError::Disconnected) if exited => break,
                    Err(IpcError::Disconnected) => {
                        error!(log, "Unexpected IPC disconnection");
                        break;
                    }
                    Err(e) => {
                        error!(log, "Unable to receive IPC message: {e}");
                        break;
                    }
                };
            }
            if let Err(e) = c2s_channel.send(C2SMessage::Disconnect {}) {
                error!(
                    slog_scope::logger(),
                    "Unable to emit ipc-message event to webview: {e}"
                );
            }
        })?;
    Ok(())
}

fn spawn_s2c_pipe(log: slog::Logger, app_handle: &AppHandle, s2c_tx: String) -> anyhow::Result<()> {
    let s2c_rx = app_handle.state::<IpcState>().s2c_rx.clone();

    let s2c_tx =
        IpcSender::<S2CMessage>::connect(s2c_tx).context("Failed to connect to S2C IPC channel")?;
    s2c_tx.send(S2CMessage::Connect)?;
    std::thread::Builder::new()
        .name("ipc-sender".to_owned())
        .spawn(move || {
            while let Ok(msg) = s2c_rx.recv() {
                if let Err(e) = s2c_tx.send(msg) {
                    error!(log, "Unable to send IPC message: {e}");
                }
            }
        })?;
    Ok(())
}

/// Inter-process communication.
pub struct Ipc {
    pub c2s_tx: IpcSender<C2SMessage>,
    pub s2c_rx: Arc<tokio::sync::Mutex<IpcReceiver<S2CMessage>>>,
}

impl Drop for Ipc {
    fn drop(&mut self) {
        _ = self.c2s_tx.send(C2SMessage::Disconnect {});
    }
}

pub struct IpcBiState<'a> {
    ipc_state: &'a IpcState,
    c2s_tx: &'a Channel<C2SMessage>,
}

impl Ipc {
    pub async fn send(&self, message: C2SMessage) -> Result<()> {
        let c2s_tx = self.c2s_tx.clone();
        Ok(tokio::task::spawn_blocking(move || c2s_tx.send(message)).await??)
    }

    pub async fn recv(&self) -> Result<S2CMessage> {
        let s2c_rx = self.s2c_rx.clone();
        Ok(tokio::task::spawn(async move {
            let s2c_rx = s2c_rx.lock().await;
            tokio::task::block_in_place(|| s2c_rx.recv())
        })
        .await??)
    }
}

impl<'a> IpcBiState<'a> {
    pub async fn send(&self, message: C2SMessage) -> Result<()> {
        let c2s_tx = self.c2s_tx.clone();
        Ok(tokio::task::spawn_blocking(move || c2s_tx.send(message)).await??)
    }

    pub async fn recv(&self) -> Result<S2CMessage> {
        Ok(self
            .ipc_state
            .s2c_rx
            .recv_async()
            .await
            .context("Channel closed")?)
    }

    // TODO: share this among Ipc and SpcReceiver
    pub async fn prompt_patient<T: Send>(
        &mut self,
        translation_key: impl Into<String>,
        message: Option<String>,
        message_args: Option<HashMap<String, serde_json::Value>>,
        fixes: impl IntoIterator<Item = DoctorFix<T>>,
    ) -> Result<T>
    where
        T: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let fixes = fixes
            .into_iter()
            .map(|fix| {
                let serde_json::Value::String(id) =
                    serde_json::to_value(fix.id).expect("Unable to serialize id")
                else {
                    panic!("Id must serialize to a string")
                };
                DoctorFix { id, ..fix }
            })
            .collect::<Vec<_>>();
        let translation_key = translation_key.into();
        let id = Uuid::new_v4();
        self.send(C2SMessage::DoctorReport(DoctorReport {
            id,
            translation_key,
            message,
            message_args,
            fixes,
        }))
        .await?;
        let response = self.recv().await?;
        let S2CMessage::PatientResponse {
            id: resp_id,
            choice,
        } = response
        else {
            bail!("Unexpected response from Manderrow: {response:?}")
        };
        if resp_id != id {
            bail!("Received a response for the wrong prompt")
        }
        Ok(serde_json::from_value(serde_json::Value::String(choice))?)
    }
}
