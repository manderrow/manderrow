pub mod commands;

use std::time::Instant;

use anyhow::{Context, Result};
use packed_semver::Version;
use simd_json::base::ValueAsScalar;
use slog::{debug, Logger};
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
) -> Result<Option<String>> {
    let start = Instant::now();
    let mut bytes = fetch_resource_as_bytes(
        app,
        log,
        reqwest,
        format!(
            "{} of mod {owner}-{name}-{version}",
            match endpoint {
                ModMarkdown::Readme => "README",
                ModMarkdown::Changelog => "CHANGELOG",
            }
        ),
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
    let tape = simd_json::to_tape(&mut bytes)?;
    let obj = tape.as_value().try_as_object()?;
    let markdown = obj
        .get("markdown")
        .context("JSON object missing required property \"markdown\"")?;
    let html = if markdown.as_null().is_some() {
        None
    } else {
        let markdown = markdown
            .as_str()
            .context("JSON property \"markdown\" is not a string")?;
        // TODO: cache
        Some(crate::util::markdown::render(markdown, |event| event)?)
    };
    let elapsed = Instant::now() - start;
    debug!(log, "Fetched and rendered mod markdown in {:?}", elapsed);
    Ok(html)
}
