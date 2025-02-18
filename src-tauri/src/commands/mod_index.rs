mod memory;

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use async_compression::tokio::bufread::GzipDecoder;
use drop_guard::ext::tokio1::JoinHandleExt;
use slog::debug;
use smol_str::SmolStr;
use tauri::{ipc::Channel, AppHandle, Manager};
use tokio::io::AsyncReadExt;
use tokio::sync::{Mutex, RwLock};
use url::Url;

use crate::games::{GAMES, GAMES_BY_ID};
use crate::mods::{ArchivedMod, Mod, ModMetadataRef, ModRef, ModVersionRef};
use crate::util::http::ResponseExt;
use crate::util::Progress;
use crate::{CommandError, Reqwest};

use memory::MemoryModIndex;

#[derive(Default)]
struct ModIndex {
    data: RwLock<Vec<MemoryModIndex>>,
    refresh_lock: Mutex<()>,
    pub progress: Progress,
}

static MOD_INDEXES: LazyLock<HashMap<&'static str, ModIndex>> = LazyLock::new(|| {
    GAMES
        .iter()
        .map(|game| (&*game.thunderstore_url, ModIndex::default()))
        .collect()
});

#[derive(Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum FetchEvent {
    Progress { completed: u32, total: u32 },
}

#[tauri::command]
pub async fn fetch_mod_index(
    app_handle: AppHandle,
    game: &str,
    refresh: bool,
    on_event: Channel<FetchEvent>,
) -> Result<(), CommandError> {
    let log = slog_scope::logger();

    let game = *GAMES_BY_ID.get(game).context("No such game")?;
    let mod_index = MOD_INDEXES.get(&*game.thunderstore_url).unwrap();

    if refresh
        || mod_index
            .data
            .try_read()
            .map(|data| data.is_empty())
            .unwrap_or(true)
    {
        debug!(log, "Fetching mods");

        let _guard = tokio::task::spawn(async move {
            loop {
                event_listener::listener!(mod_index.progress.updates() => listener);

                let (completed, total) = mod_index.progress.get();
                _ = on_event.send(FetchEvent::Progress { completed, total });

                listener.await;

                let (completed, total) = mod_index.progress.get();
                _ = on_event.send(FetchEvent::Progress { completed, total });
            }
        })
        .abort_on_drop();

        let Ok(_lock) = mod_index.refresh_lock.try_lock() else {
            // just wait for the current refetch to complete.
            _ = mod_index.refresh_lock.lock().await;
            return Ok(());
        };

        mod_index.progress.set(0, 1);
        let mut chunk_urls = Vec::new();
        GzipDecoder::new(
            app_handle
                .state::<Reqwest>()
                .get(&game.thunderstore_url)
                .send()
                .await
                .context("Failed to fetch chunk URLs from Thunderstore")?
                .error_for_status()
                .context("Failed to fetch chunk URLs from Thunderstore")?
                .reader(),
        )
        .read_to_end(&mut chunk_urls)
        .await
        .context("Failed to fetch chunk URLs from Thunderstore")?;
        let chunk_urls =
            tokio::task::block_in_place(|| simd_json::from_slice::<Vec<Url>>(&mut chunk_urls))
                .context("Unable to decode chunk URLs from Thunderstore")?;
        mod_index.progress.inc(
            1,
            chunk_urls.len().try_into().context("too many chunk urls")?,
        );

        let started_at = std::time::Instant::now();
        let new_mod_index =
            futures::future::try_join_all(chunk_urls.into_iter().map(|url| async {
                let log = log.clone();
                let app_handle = app_handle.clone();
                tokio::task::spawn(async move {
                    let spawned_at = std::time::Instant::now();
                    let latency = spawned_at.duration_since(started_at);
                    let mut buf = Vec::new();
                    {
                        let mut rdr = GzipDecoder::new(
                            app_handle
                                .state::<Reqwest>()
                                .get(url)
                                .send()
                                .await
                                .context("Failed to fetch chunk from Thunderstore")?
                                .error_for_status()
                                .context("Failed to fetch chunk from Thunderstore")?
                                .reader_with_progress(&mod_index.progress),
                        );
                        rdr.read_to_end(&mut buf).await?;
                    }
                    let fetched_at = std::time::Instant::now();
                    let fetched_in = fetched_at.duration_since(spawned_at);
                    tokio::task::block_in_place(move || {
                        // TODO: decode without cloning
                        let mods = simd_json::from_slice::<Vec<Mod>>(&mut buf)?;
                        let buf_len = buf.len();
                        drop(buf);
                        let decoded_at = std::time::Instant::now();
                        let decoded_in = decoded_at.duration_since(fetched_at);
                        let mods = rkyv::util::with_arena(|arena| {
                            let mut serializer = rkyv_intern::InterningAdapter::new(
                                rkyv::ser::Serializer::new(
                                    rkyv::util::AlignedVec::<16>::new(),
                                    arena.acquire(),
                                    rkyv::ser::sharing::Share::new(),
                                ),
                                rkyv_intern::Interner::<SmolStr>::default(),
                            );
                            rkyv::api::serialize_using::<_, rkyv::rancor::Error>(
                                &mods,
                                &mut serializer,
                            )?;
                            Ok::<_, rkyv::rancor::Error>(serializer.into_serializer().into_writer())
                        })?;
                        let encoded_at = std::time::Instant::now();
                        let encoded_in = encoded_at.duration_since(decoded_at);
                        debug!(
                            log,
                            "{buf_len} bytes of JSON -> {} bytes in memory ({:.2}%), {latency:?} spawning, {fetched_in:?} fetching, {decoded_in:?} decoding, {encoded_in:?} encoding",
                            mods.len(),
                            (mods.len() as f64 / buf_len as f64) * 100.0
                        );
                        let index = MemoryModIndex::new(mods, |data| {
                            if cfg!(debug_assertions) {
                                rkyv::access::<_, rkyv::rancor::Error>(data)
                            } else{
                                // SAFETY: rkyv just gave us this data. We trust it.
                                Ok(unsafe { rkyv::access_unchecked(data) })
                            }
                        })?;
                        mod_index.progress.inc(1, 0);
                        Ok::<_, anyhow::Error>(index)
                    })
                })
                .await?
            }))
            .await?;
        *mod_index.data.write().await = new_mod_index;

        debug!(log, "Finished fetching mods");
    }

    Ok(())
}

#[derive(Clone, Copy, serde::Deserialize)]
pub enum SortColumn {
    Relevance,
    Name,
    Owner,
    Downloads,
}

#[derive(Clone, Copy, serde::Deserialize)]
pub struct SortOption {
    column: SortColumn,
    descending: bool,
}

// #[derive(serde::Serialize)]
// pub struct QueryResult<'a> {
//     mods: Vec<&'a Mod<'a>>,
//     count: usize,
// }

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct ModId<'a> {
    owner: &'a str,
    name: &'a str,
}

// TODO: use register_asynchronous_uri_scheme_protocol to stream the json back without buffering
#[tauri::command]
pub async fn query_mod_index(
    game: &str,
    query: &str,
    sort: Vec<SortOption>,
    skip: Option<usize>,
    limit: Option<usize>,
    exact: Option<HashSet<ModId<'_>>>,
) -> Result<simd_json::OwnedValue, CommandError> {
    let log = slog_scope::logger();

    let game = *GAMES_BY_ID.get(game).context("No such game")?;
    let mod_index = MOD_INDEXES
        .get(&*game.thunderstore_url)
        .unwrap()
        .data
        .read()
        .await;

    debug!(log, "Querying mods");

    let mut buf = mod_index
        .iter()
        .flat_map(|mi| {
            mi.mods()
                .iter()
                .filter(|m| {
                    if let Some(exact) = &exact {
                        exact.contains(&ModId {
                            owner: m.owner.as_str(),
                            name: m.name.as_str(),
                        })
                    } else {
                        true
                    }
                })
                .filter_map(|m| {
                    if query.is_empty() {
                        Some((m, 0.0))
                    } else {
                        let (_, owner_score) = rff::match_and_score(&query, &m.owner)?;
                        let (_, name_score) = rff::match_and_score(&query, &m.name)?;
                        Some((m, owner_score + name_score))
                    }
                })
        })
        .collect::<Vec<_>>();
    buf.sort_unstable_by(|(m1, score1), (m2, score2)| {
        let mut ordering = std::cmp::Ordering::Equal;
        for &SortOption { column, descending } in &sort {
            ordering = match column {
                SortColumn::Relevance => score1.total_cmp(score2),
                SortColumn::Name => m1.name.cmp(&m2.name),
                SortColumn::Owner => m1.owner.cmp(&m2.owner),
                SortColumn::Downloads => {
                    let sum_downloads = |m: &ArchivedMod| {
                        m.versions
                            .iter()
                            .map(|v| u64::from(v.downloads))
                            .sum::<u64>()
                    };
                    sum_downloads(m1).cmp(&sum_downloads(m2))
                }
            };
            if descending {
                ordering = ordering.reverse();
            }
            if ordering.is_ne() {
                break;
            }
        }
        ordering
    });

    let count = buf.len();

    fn map_to_json<'a>(
        it: impl IntoIterator<Item = (&'a ArchivedMod, f64)>,
    ) -> Vec<simd_json::OwnedValue> {
        it.into_iter()
            .map(|(m, _)| {
                simd_json::serde::to_owned_value(ModRef {
                    metadata: ModMetadataRef {
                        name: &m.name,
                        full_name: Default::default(),
                        owner: &m.owner,
                        package_url: Default::default(),
                        donation_link: m.donation_link.as_deref(),
                        date_created: &m.date_created,
                        date_updated: &m.date_updated,
                        rating_score: m.rating_score.into(),
                        is_pinned: m.is_pinned,
                        is_deprecated: m.is_deprecated,
                        has_nsfw_content: m.has_nsfw_content,
                        categories: m.categories.iter().map(|s| s.0.as_str()).collect(),
                        uuid4: m.uuid4,
                    },
                    versions: m
                        .versions
                        .iter()
                        .map(|v| ModVersionRef {
                            name: Default::default(),
                            full_name: Default::default(),
                            description: &v.description,
                            icon: Default::default(),
                            version_number: v.version_number.into(),
                            dependencies: v.dependencies.iter().map(|s| s.0.as_str()).collect(),
                            download_url: Default::default(),
                            downloads: v.downloads.into(),
                            date_created: &v.date_created,
                            website_url: v.website_url.as_deref(),
                            is_active: v.is_active.into(),
                            uuid4: v.uuid4,
                            file_size: v.file_size.into(),
                        })
                        .collect(),
                })
                .unwrap()
            })
            .collect::<Vec<_>>()
    }

    let mut map = simd_json::owned::Object::with_capacity(2);
    map.insert_nocheck("count".to_owned(), simd_json::OwnedValue::from(count));
    let buf = match (skip, limit) {
        (Some(skip), Some(limit)) => map_to_json(buf.into_iter().skip(skip).take(limit)),
        (None, Some(limit)) => map_to_json(buf.into_iter().take(limit)),
        (Some(skip), None) => map_to_json(buf.into_iter().skip(skip)),
        (None, None) => map_to_json(buf),
    };
    map.insert_nocheck(
        "mods".to_owned(),
        simd_json::OwnedValue::Array(Box::new(buf)),
    );
    Ok(simd_json::OwnedValue::Object(Box::new(map)))
}
