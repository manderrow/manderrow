mod memory;

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use async_compression::tokio::bufread::GzipDecoder;
use drop_guard::ext::tokio1::JoinHandleExt;
use rkyv_intern::Interner;
use slog::{debug, info};
use tauri::{ipc::Channel, AppHandle, Manager};
use tokio::io::AsyncReadExt;
use tokio::sync::{Mutex, RwLock};
use url::Url;

use crate::games::{GAMES, GAMES_BY_ID};
use crate::mods::{ArchivedModRef, ModRef};
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
    Progress {
        completed_steps: u32,
        total_steps: u32,
        progress: f64,
    },
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
        info!(log, "Fetching mods");

        let _guard = tokio::task::spawn(async move {
            loop {
                let (completed_steps, total_steps) = mod_index.progress.get_steps();
                let (completed_progress, total_progress) = mod_index.progress.get_progress();
                _ = on_event.send(FetchEvent::Progress {
                    completed_steps,
                    total_steps,
                    progress: if total_progress == 0 {
                        0.0
                    } else {
                        completed_progress as f64 / total_progress as f64
                    },
                });

                mod_index.progress.updates().notified().await;
            }
        })
        .abort_on_drop();

        let Ok(_lock) = mod_index.refresh_lock.try_lock() else {
            // just wait for the current refetch to complete.
            _ = mod_index.refresh_lock.lock().await;
            return Ok(());
        };

        mod_index.progress.reset();

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
                .reader_with_progress(&mod_index.progress),
        )
        .read_to_end(&mut chunk_urls)
        .await
        .context("Failed to fetch chunk URLs from Thunderstore")?;
        let chunk_urls =
            tokio::task::block_in_place(|| simd_json::from_slice::<Vec<Url>>(&mut chunk_urls))
                .context("Unable to decode chunk URLs from Thunderstore")?;

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
                        let _step = mod_index.progress.step();
                        let mut rdr = GzipDecoder::new(
                            app_handle
                                .state::<Reqwest>()
                                .get(url.clone())
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
                    let _step = mod_index.progress.step();
                    tokio::task::block_in_place(move || {
                        let buf_len = buf.len();
                        let mods = simd_json::from_slice::<Vec<ModRef>>(&mut buf)?;
                        let decoded_at = std::time::Instant::now();
                        let decoded_in = decoded_at.duration_since(fetched_at);

                        #[derive(Default)]
                        struct Statistics {
                            values: usize,
                            total_bytes: usize,
                            average_uses: f64,
                            single_use_entries: usize,
                        }
                        impl std::fmt::Display for Statistics {
                            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                                let Statistics { values, total_bytes, average_uses, single_use_entries } = self;
                                write!(f, "{values} strings interned, {total_bytes} bytes, avg. {average_uses} uses/string, {single_use_entries} single-use strings")
                            }
                        }

                        let (buf, stats) = rkyv::util::with_arena(|arena| {
                            let mut serializer = rkyv_intern::InterningAdapter::new(
                                rkyv_intern::InterningAdapter::new(
                                    rkyv::ser::Serializer::new(
                                        rkyv::util::AlignedVec::<16>::with_capacity(buf_len / 4),
                                        arena.acquire(),
                                        rkyv::ser::sharing::Share::new(),
                                    ),
                                    Interner::<ModId<'_>>::default(),
                                ),
                                Interner::<String>::default(),
                            );
                            rkyv::api::serialize_using::<_, rkyv::rancor::Error>(
                                &mods,
                                &mut serializer,
                            )?;
                            let (serializer, interner) = serializer.into_components();
                            #[derive(Default)]
                            struct StatisticsAccumulator {
                                total_bytes: usize,
                                total_uses: usize,
                                single_use_entries: usize,
                            }
                            let stats = interner.iter().map(|(s, e)| (s.len(), e.ref_cnt.get())).fold(StatisticsAccumulator::default(), |mut stats, (len, ref_cnt)| {
                                stats.total_bytes += len;
                                stats.total_uses += ref_cnt;
                                if ref_cnt == 1 {
                                    stats.single_use_entries += 1;
                                }
                                stats
                            });
                            Ok::<_, rkyv::rancor::Error>((serializer.into_serializer().into_writer(), Statistics {
                                values: interner.len(),
                                total_bytes: stats.total_bytes,
                                average_uses: stats.total_uses as f64 / interner.len() as f64,
                                single_use_entries: stats.single_use_entries,
                            }))
                        })?;
                        let encoded_at = std::time::Instant::now();
                        let encoded_in = encoded_at.duration_since(decoded_at);
                        info!(
                            log,
                            "{buf_len} bytes of JSON -> {} bytes in memory ({:.2}%, {stats}), {latency:?} spawning, {fetched_in:?} fetching, {decoded_in:?} decoding, {encoded_in:?} encoding",
                            buf.len(),
                            (buf.len() as f64 / buf_len as f64) * 100.0
                        );
                        let index = MemoryModIndex::new(buf, |data| {
                            if cfg!(debug_assertions) {
                                rkyv::access::<_, rkyv::rancor::Error>(data)
                            } else{
                                // SAFETY: rkyv just gave us this data. We trust it.
                                Ok(unsafe { rkyv::access_unchecked(data) })
                            }
                        }).with_context(|| format!("Failed to create mod index from chunk at {url:?}"))?;
                        Ok::<_, anyhow::Error>(index)
                    })
                })
                .await?
            }))
            .await?;
        *mod_index.data.write().await = new_mod_index;

        info!(log, "Finished fetching mods");
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

#[tauri::command]
pub async fn query_mod_index(
    game: &str,
    query: &str,
    sort: Vec<SortOption>,
    skip: Option<usize>,
    limit: Option<usize>,
    exact: Option<HashSet<ModId<'_>>>,
) -> Result<tauri::ipc::Response, CommandError> {
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
                            owner: &*m.owner,
                            name: &*m.name,
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
                    let sum_downloads = |m: &ArchivedModRef| {
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

    fn map_to_json<'a>(buf: &mut Vec<u8>, it: impl Iterator<Item = &'a ArchivedModRef<'a>>) {
        let mut it = it.peekable();
        while let Some(m) = it.next() {
            simd_json::serde::to_writer(&mut *buf, m).unwrap();
            if it.peek().is_some() {
                buf.push(b',');
            }
        }
    }

    let mut out_buf = br#"{"count":"#.as_slice().to_owned();
    simd_json::serde::to_writer(&mut out_buf, &count).unwrap();
    out_buf.extend(br#","mods":["#);
    let mods = buf.into_iter().map(|(m, _)| m);
    match (skip, limit) {
        (Some(skip), Some(limit)) => map_to_json(&mut out_buf, mods.skip(skip).take(limit)),
        (None, Some(limit)) => map_to_json(&mut out_buf, mods.take(limit)),
        (Some(skip), None) => map_to_json(&mut out_buf, mods.skip(skip)),
        (None, None) => map_to_json(&mut out_buf, mods),
    };
    out_buf.extend(b"]}");
    // SAFETY: simd_json only writes valid UTF-8
    Ok(tauri::ipc::Response::new(unsafe {
        String::from_utf8_unchecked(out_buf)
    }))
}
