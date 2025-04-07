use std::collections::HashMap;
use std::ops::ControlFlow;

use anyhow::{Context, Result};
use manderrow_ipc::ipc_channel::ipc::{IpcError, IpcOneShotServer, IpcSender};
use slog::error;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

pub use manderrow_ipc::*;

pub trait InProcessIpcStateExt {
    fn bi<'a>(&'a self, channel: &'a Channel<C2SMessage>) -> IpcBiState<'a>;
}

impl InProcessIpcStateExt for IpcState {
    fn bi<'a>(&'a self, channel: &'a Channel<C2SMessage>) -> IpcBiState<'a> {
        IpcBiState {
            ipc_state: self,
            c2s_tx: channel,
        }
    }
}

pub struct IpcBiState<'a> {
    ipc_state: &'a IpcState,
    c2s_tx: &'a Channel<C2SMessage>,
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
        &self,
        translation_key: impl Into<String>,
        message: Option<String>,
        message_args: Option<HashMap<String, serde_json::Value>>,
        fixes: impl IntoIterator<Item = DoctorFix<T>>,
    ) -> Result<T>
    where
        T: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let (mut receiver, msg) =
            PatientChoiceReceiver::new(translation_key, message, message_args, fixes);
        self.send(msg).await?;
        loop {
            match receiver.process(self.recv().await?)? {
                ControlFlow::Break(choice) => return Ok(choice),
                ControlFlow::Continue(r) => receiver = r,
            }
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
