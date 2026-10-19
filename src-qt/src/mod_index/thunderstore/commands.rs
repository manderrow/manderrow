use packed_semver::Version;

use crate::app::{AppHandle, RawJson};
use crate::{tasks, CommandError, Reqwest};

use super::ModMarkdown;

pub async fn thunderstore_fetch_mod_markdown(
    app: &AppHandle,
    reqwest: &Reqwest,
    owner: &str,
    name: &str,
    version: Version,
    endpoint: ModMarkdown,
    task_id: tasks::Id,
) -> Result<RawJson, CommandError> {
    super::fetch_mod_markdown(
        Some(app),
        &slog_scope::logger(),
        reqwest,
        owner,
        name,
        version,
        endpoint,
        Some(task_id),
    )
    .await
    .map_err(Into::into)
    .map(RawJson)
}
