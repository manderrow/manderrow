use anyhow::Context as _;
use tauri::State;

use crate::ipc::{IpcState, S2CMessage};
use crate::CommandError;

#[tauri::command]
pub async fn send_s2c_message(
    ipc_state: State<'_, IpcState>,
    msg: S2CMessage,
) -> Result<(), CommandError> {
    let s2c_tx = ipc_state.s2c_tx.read().await;
    if let Some(s2c_tx) = &*s2c_tx {
        s2c_tx
            .send(msg)
            .await
            .context("Failed to send IPC message")?;
    }
    Ok(())
}
