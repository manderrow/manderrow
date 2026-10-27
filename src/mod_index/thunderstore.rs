use anyhow::{Context as _, Result};
use packed_semver::Version;
use slog::Logger;
use crate::app::AppHandle;

use crate::installing::{fetch_resource_as_bytes, CacheOptions};
use crate::{tasks, Reqwest};

/// A Markdown document that a version of a mod may have.
#[derive(Debug, Clone, Copy)]
pub enum ModMarkdown {
    Readme,
    Changelog,
}

/// Fetches a Markdown document of a version of a mod, which is `None` if it has none.
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
    let bytes = fetch_resource_as_bytes(
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
    #[derive(serde::Deserialize)]
    struct Response {
        markdown: Option<String>,
    }
    let response =
        serde_json::from_slice::<Response>(&bytes).context("Invalid response from Thunderstore")?;
    Ok(response.markdown)
}
