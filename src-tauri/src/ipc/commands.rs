use anyhow::{anyhow, Context};
use tauri::State;

use crate::ipc::{ConnectionId, IpcState, S2CMessage};
use crate::CommandError;

#[tauri::command]
pub async fn allocate_ipc_connection(
    ipc_state: State<'_, IpcState>,
) -> Result<ConnectionId, CommandError> {
    Ok(ipc_state.alloc())
}

#[tauri::command]
pub async fn send_s2c_message(
    ipc_state: State<'_, IpcState>,
    conn_id: ConnectionId,
    msg: S2CMessage,
) -> Result<(), CommandError> {
    let Some(conn) = ipc_state.get_conn(conn_id) else {
        return Err(anyhow!("No such connection: {conn_id:?}").into());
    };
    conn.send_async(msg)
        .await
        .context("Failed to send IPC message")?;
    Ok(())
}

#[tauri::command]
pub async fn get_ipc_connections(
    ipc_state: State<'_, IpcState>,
) -> Result<Vec<ConnectionId>, CommandError> {
    Ok(ipc_state.get_conns())
}

#[tauri::command]
pub async fn kill_ipc_client(
    ipc_state: State<'_, IpcState>,
    conn_id: ConnectionId,
) -> Result<(), CommandError> {
    let log = slog_scope::logger();
    let Some(conn) = ipc_state.get_conn(conn_id) else {
        return Err(anyhow!("No such connection: {conn_id:?}").into());
    };
    conn.kill_process(&log)
        .context("Failed to kill IPC client")?;
    Ok(())
}
