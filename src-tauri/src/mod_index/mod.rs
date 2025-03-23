pub mod commands;
mod memory;

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use async_compression::tokio::bufread::GzipDecoder;
use rkyv_intern::Interner;
use slog::{debug, info};
use tauri::{AppHandle, Manager};
use tokio::io::AsyncReadExt;
use tokio::select;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};
use url::Url;

use crate::games::{GAMES, GAMES_BY_ID};
use crate::mods::{ArchivedModRef, ModId, ModRef};
use crate::tasks::{self, TaskBuilder};
use crate::util::http::ResponseExt;
use crate::util::rkyv::InternedString;
use crate::util::search::{Score, SortOption};
use crate::util::{search, Progress};
use crate::Reqwest;

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

pub async fn fetch_mod_index(
    app: &AppHandle,
    game: &str,
    refresh: bool,
    task_id: Option<tasks::Id>,
) -> Result<()> {
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
        TaskBuilder::with_id(task_id.unwrap_or_else(tasks::allocate_task), format!("Fetch mod index for {}", game.id))
            .progress_unit(tasks::ProgressUnit::Bytes)
            .run_with_handle(Some(app), |handle| async move {
                info!(log, "Fetching mods");

                let Ok(_lock) = mod_index.refresh_lock.try_lock() else {
                    // just wait for the current refetch to complete.
                    _ = mod_index.refresh_lock.lock().await;
                    return Ok(());
                };

                #[cfg(feature = "statistics")]
                crate::mods::reset_version_repr_stats();

                mod_index.progress.reset();

                let progress_updater = async {
                    loop {
                        _ = handle.send_progress(app, &mod_index.progress);

                        mod_index.progress.updates().notified().await;
                    }
                };

                let new_mod_index = async {
                    let mut chunk_urls = Vec::new();
                    GzipDecoder::new(
                        app
                            .state::<Reqwest>()
                            .get(&*game.thunderstore_url)
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

                    futures::future::try_join_all(chunk_urls.into_iter().map(|url| async {
                        let log = log.clone();
                        let app_handle = app.clone();
                        tokio::task::spawn(async move {
                            let spawned_at = std::time::Instant::now();
                            let latency = spawned_at.duration_since(started_at);
                            let mut buf = Vec::new();
                            {
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
                            tokio::task::block_in_place(move || {
                                let buf_len = buf.len();
                                let mods = simd_json::from_slice::<Vec<ModRef>>(&mut buf)?;
                                let decoded_at = std::time::Instant::now();
                                let decoded_in = decoded_at.duration_since(fetched_at);

                                #[cfg(feature = "statistics")]
                                #[derive(Default)]
                                struct Statistics {
                                    values: usize,
                                    total_bytes: usize,
                                    average_uses: f64,
                                    single_use_entries: usize,
                                }
                                #[cfg(not(feature = "statistics"))]
                                struct Statistics;
                                impl std::fmt::Display for Statistics {
                                    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                                        #[cfg(feature = "statistics")]
                                        {
                                            let Statistics { values, total_bytes, average_uses, single_use_entries } = self;
                                            write!(_f, "{values} strings interned, {total_bytes} bytes, avg. {average_uses} uses/string, {single_use_entries} single-use strings")?;
                                        }
                                        Ok(())
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
                                    let (serializer, _interner) = serializer.into_components();
                                    #[cfg(feature = "statistics")]
                                    #[derive(Default)]
                                    struct StatisticsAccumulator {
                                        total_bytes: usize,
                                        total_uses: usize,
                                        single_use_entries: usize,
                                    }
                                    #[cfg(feature = "statistics")]
                                    let stats = _interner.iter().map(|(s, e)| (s.len(), e.ref_cnt.get())).fold(StatisticsAccumulator::default(), |mut stats, (len, ref_cnt)| {
                                        stats.total_bytes += len;
                                        stats.total_uses += ref_cnt;
                                        if ref_cnt == 1 {
                                            stats.single_use_entries += 1;
                                        }
                                        stats
                                    });
                                    Ok::<_, rkyv::rancor::Error>((serializer.into_serializer().into_writer(), {
                                        #[cfg(feature = "statistics")]
                                        {
                                            Statistics {
                                                values: _interner.len(),
                                                total_bytes: stats.total_bytes,
                                                average_uses: stats.total_uses as f64 / _interner.len() as f64,
                                                single_use_entries: stats.single_use_entries,
                                            }
                                        }
                                        #[cfg(not(feature = "statistics"))]
                                        {
                                            Statistics
                                        }
                                    }))
                                })?;
                                let encoded_at = std::time::Instant::now();
                                let encoded_in = encoded_at.duration_since(decoded_at);
                                let stats_prefix = if cfg!(feature = "statistics") { ", " } else { "" };
                                info!(
                                    log,
                                    "{buf_len} bytes of JSON -> {} bytes in memory ({:.2}%{stats_prefix}{stats}), {latency:?} spawning, {fetched_in:?} fetching, {decoded_in:?} decoding, {encoded_in:?} encoding",
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
                    })).await
                };
                let new_mod_index = select! {
                    // The "fair" strategy employed by select! should be entirely unnecessary for
                    // this particular use case. `progress_updater` never polls Ready, so it cannot
                    // starve new_mod_index.
                    biased;
                    _ = progress_updater => unreachable!(),
                    r = new_mod_index => r?,
                };
                *mod_index.data.write().await = new_mod_index;

                #[cfg(feature = "statistics")]
                let (inline_version_count, out_of_line_version_count) =
                    crate::mods::get_version_repr_stats();
                #[cfg(not(feature = "statistics"))]
                let (inline_version_count, out_of_line_version_count) = (None::<u32>, None::<u32>);
                info!(log, "Finished fetching mods"; "inline_version_count" => inline_version_count, "out_of_line_version_count" => out_of_line_version_count);

                Ok::<_, anyhow::Error>(())
            })
            .await
            .map_err(Into::into)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, serde::Deserialize)]
pub enum SortColumn {
    Relevance,
    Name,
    Owner,
    Downloads,
}

pub type ModIndexReadGuard = RwLockReadGuard<'static, Vec<MemoryModIndex>>;

pub async fn read_mod_index(game: &str) -> Result<ModIndexReadGuard> {
    let game = *GAMES_BY_ID.get(game).context("No such game")?;
    Ok(MOD_INDEXES
        .get(&*game.thunderstore_url)
        .unwrap()
        .data
        .read()
        .await)
}

pub async fn count_mod_index<'a>(mod_index: &'a ModIndexReadGuard, query: &str) -> Result<usize> {
    let log = slog_scope::logger();

    debug!(log, "Counting mods");

    Ok(mod_index
        .iter()
        .flat_map(|mi| {
            mi.mods()
                .iter()
                .filter_map(|m| score_mod(&log, query, m))
                .filter(|&(_, score)| search::should_include(score))
        })
        .count())
}

pub async fn query_mod_index<'a>(
    mod_index: &'a ModIndexReadGuard,
    query: &str,
    sort: &[SortOption<SortColumn>],
) -> Result<Vec<(&'a ArchivedModRef<'a>, Score)>> {
    let log = slog_scope::logger();

    debug!(log, "Querying mods");

    let mut buf = mod_index
        .iter()
        .flat_map(|mi| {
            mi.mods()
                .iter()
                .filter_map(|m| score_mod(&log, query, m))
                .filter(|&(_, score)| search::should_include(score))
        })
        .collect::<Vec<_>>();
    if !sort.is_empty() {
        buf.sort_unstable_by(|(m1, score1), (m2, score2)| {
            let mut ordering = std::cmp::Ordering::Equal;
            for &SortOption { column, descending } in sort {
                ordering = match column {
                    SortColumn::Relevance => score1.cmp(score2),
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
    }

    Ok(buf)
}

fn score_mod<'a, 'b>(
    _log: &slog::Logger,
    query: &str,
    m: &'a ArchivedModRef<'b>,
) -> Option<(&'a ArchivedModRef<'b>, Score)> {
    if query.is_empty() {
        Some((m, Score::MAX))
    } else {
        let owner_score =
            search::score(&query, &m.owner).map(|s| std::cmp::max(s / 8, Score::ZERO));
        let name_score = search::score(&query, &m.name);
        let score = search::add_scores(name_score, owner_score)?;
        Some((m, score))
    }
}

pub async fn get_from_mod_index<'a>(
    mod_index: &'a ModIndexReadGuard,
    mod_ids: &HashSet<ModId<'_>>,
) -> Result<Vec<&'a ArchivedModRef<'a>>> {
    let log = slog_scope::logger();

    debug!(log, "Querying mods");

    let buf = mod_index
        .iter()
        .flat_map(|mi| {
            mi.mods().iter().filter(|m| {
                mod_ids.contains(&ModId {
                    owner: InternedString(&*m.owner),
                    name: InternedString(&*m.name),
                })
            })
        })
        .collect::<Vec<_>>();

    Ok(buf)
}
