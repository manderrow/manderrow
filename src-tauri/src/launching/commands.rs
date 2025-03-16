use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::ipc::{C2SMessage, IpcState, S2CMessage};
use crate::CommandError;

#[tauri::command]
pub async fn send_s2c_message(
    ipc_state: State<'_, IpcState>,
    msg: S2CMessage,
) -> Result<(), CommandError> {
    super::send_s2c_message(&*ipc_state, msg)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn launch_profile(
    app_handle: AppHandle,
    ipc_state: State<'_, IpcState>,
    id: Uuid,
    modded: bool,
    channel: Channel<C2SMessage>,
) -> Result<(), CommandError> {
    super::launch_profile(app_handle, &*ipc_state, id, modded, channel)
        .await
        .map_err(Into::into)
}
