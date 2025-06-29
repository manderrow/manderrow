pub mod commands;

use anyhow::Result;
use packed_semver::Version;
use slog::Logger;
use tauri::AppHandle;

use crate::installing::{fetch_resource_as_bytes, CacheOptions};
use crate::{tasks, Reqwest};

#[derive(Clone, Copy, serde::Deserialize)]
pub enum ModMarkdown {
    #[serde(rename = "readme")]
    Readme,
    #[serde(rename = "changelog")]
    Changelog,
}

pub async fn fetch_mod_markdown(
    app: Option<&AppHandle>,
    log: &Logger,
    reqwest: &Reqwest,
    owner: &str,
    name: &str,
    version: Version,
    endpoint: ModMarkdown,
    task_id: Option<tasks::Id>,
) -> Result<String> {
    let bytes = fetch_resource_as_bytes(
        app,
        log,
        reqwest,
        &format!(
            "https://thunderstore.io/api/experimental/package/{owner}/{name}/{version}/{}/",
            match endpoint {
                ModMarkdown::Readme => "readme",
                ModMarkdown::Changelog => "changelog",
            }
        ),
        Some(CacheOptions::by_url()),
        task_id,
    )
    .await?;
    Ok(String::from_utf8(Vec::from(bytes))?)
}
