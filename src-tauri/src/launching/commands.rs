use tauri::{AppHandle, State};

use crate::ipc::{ConnectionId, IpcState};
use crate::CommandError;

use super::LaunchTarget;

#[tauri::command]
pub async fn launch_profile(
    app: AppHandle,
    ipc_state: State<'_, IpcState>,
    target: LaunchTarget<'_>,
    modded: bool,
    conn_id: ConnectionId,
) -> Result<(), CommandError> {
    super::launch_profile(app, &*ipc_state, target, modded, conn_id)
        .await
        .map_err(Into::into)
}
