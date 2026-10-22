use anyhow::{anyhow, Context};

use crate::ipc::{ConnectionId, IpcState, S2CMessage};
use crate::CommandError;

pub async fn allocate_ipc_connection(
    ipc_state: &IpcState,
) -> Result<ConnectionId, CommandError> {
    Ok(ipc_state.alloc())
}

pub async fn send_s2c_message(
    ipc_state: &IpcState,
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

pub async fn get_ipc_connections(
    ipc_state: &IpcState,
) -> Result<Vec<ConnectionId>, CommandError> {
    Ok(ipc_state.get_conns())
}

pub async fn kill_ipc_client(
    ipc_state: &IpcState,
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
