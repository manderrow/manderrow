use anyhow::Context;

use crate::CommandError;

use super::Id;

#[tauri::command]
pub async fn allocate_task() -> Result<Id, CommandError> {
    Ok(super::allocate_task())
}

#[tauri::command]
pub async fn cancel_task(id: Id) -> Result<(), CommandError> {
    let cancel = {
        let mut tasks = super::TASKS.write().await;
        tasks.get_mut(&id).context("No such task")?.cancel.take()
    };
    if let Some(cancel) = cancel {
        // Failure just means the task has already completed. Ignore it.
        _ = cancel.send(());
    } else {
        // The task has already been cancelled. Ignore.
    }
    Ok(())
}
