use packed_semver::Version;
use tauri::{AppHandle, State};

use crate::{tasks, CommandError, Reqwest};

use super::ModMarkdown;

#[tauri::command]
pub async fn thunderstore_fetch_mod_markdown(
    app: AppHandle,
    reqwest: State<'_, Reqwest>,
    owner: &str,
    name: &str,
    version: Version,
    endpoint: ModMarkdown,
    task_id: tasks::Id,
) -> Result<Option<String>, CommandError> {
    super::fetch_mod_markdown(
        Some(&app),
        &slog_scope::logger(),
        &reqwest,
        owner,
        name,
        version,
        endpoint,
        Some(task_id),
    )
    .await
    .map_err(Into::into)
}
