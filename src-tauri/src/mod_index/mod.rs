pub mod commands;
mod memory;
pub mod thunderstore;

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Instant;

use anyhow::{Context as _, Result};
use async_compression::tokio::bufread::GzipDecoder;
use bumpalo::Bump;
use manderrow_types::mods::{ArchivedModRef, ModId, ModRef};
use manderrow_types::util::rkyv::InternedString;
use rkyv_intern::Interner;
use simple_sublime_fuzzy::Query;
use slog::{debug, info, trace};
use tauri::AppHandle;
use tokio::io::AsyncReadExt;
use tokio::select;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};
use url::Url;

use crate::games::{games, games_by_id};
use crate::tasks::{self, TaskBuilder};
use crate::util::http::ResponseExt;
use crate::util::search::{Needle, Score, SortOption};
use crate::util::{search, Progress};
use crate::Reqwest;

use memory::{MemoryModIndex, MemoryModIndexChunk};

#[derive(Default)]
struct ModIndex {
    data: RwLock<MemoryModIndex>,
    refresh_lock: Mutex<()>,
    pub progress: Progress,
}

static MOD_INDEXES: LazyLock<HashMap<&'static str, ModIndex>> = LazyLock::new(|| {
    let Ok(games) = games() else {
        return HashMap::new();
    };
    games
        .iter()
        .map(|game| (&*game.thunderstore_url, ModIndex::default()))
        .collect()
});

pub async fn fetch_mod_index(
    app: Option<&AppHandle>,
    reqwest: &Reqwest,
    game: &str,
    refresh: bool,
    task_id: Option<tasks::Id>,
) -> Result<()> {
    let log = slog_scope::logger();

    let game = *games_by_id()?.get(game).context("No such game")?;
    let mod_index = MOD_INDEXES.get(&*game.thunderstore_url).unwrap();

    // TODO: document when chunks can be empty
    if refresh
        || mod_index
            .data
            .try_read()
            .map(|data| data.chunks.is_empty())
            .unwrap_or(true)
    {
        TaskBuilder::with_id(task_id.unwrap_or_else(tasks::allocate_task), format!("Fetch mod index for {}", game.id))
            .progress_unit(tasks::ProgressUnit::Bytes)
            .run_with_handle(app, |handle| async move {
                info!(log, "Fetching mods");

                let Ok(_lock) = mod_index.refresh_lock.try_lock() else {
                    // just wait for the current refetch to complete.
                    _ = mod_index.refresh_lock.lock().await;
                    return Ok((None, ()));
                };

                #[cfg(feature = "statistics")]
                packed_semver::reset_version_repr_stats();

                mod_index.progress.reset();

                let progress_updater = async {
                    if let Some(app) = app {
                        loop {
                            _ = handle.send_progress(app, &mod_index.progress);

                            mod_index.progress.updates().notified().await;
                        }
                    } else {
                        std::future::pending().await
                    }
                };

                let new_mod_index = async {
                    let mut chunk_urls = Vec::new();
                    GzipDecoder::new(
                        reqwest
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

                    futures_util::future::try_join_all(chunk_urls.into_iter().map(|url| async {
                        let log = log.clone();
                        let reqwest = reqwest.clone();
                        tokio::task::spawn(async move {
                            let spawned_at = std::time::Instant::now();
                            let latency = spawned_at.duration_since(started_at);
                            let mut buf = Vec::new();
                            {
                                let mut rdr = GzipDecoder::new(
                                    reqwest
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
                                // TODO: rkyv serialize from simd_json tape directly, validating
                                //       as we go
                                let mut mods = simd_json::from_slice::<Vec<ModRef>>(&mut buf)?;
                                let decoded_at = std::time::Instant::now();
                                let decoded_in = decoded_at.duration_since(fetched_at);

                                for m in &mut mods {
                                    m.total_downloads = m.versions
                                        .iter()
                                        .map(|v| u64::from(v.downloads))
                                        .sum::<u64>();
                                }

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
                                let index = MemoryModIndexChunk::new(buf, |data| {
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
                *mod_index.data.write().await = MemoryModIndex { chunks: new_mod_index };

                #[cfg(feature = "statistics")]
                let (inline_version_count, out_of_line_version_count) = packed_semver::get_version_repr_stats();
                #[cfg(not(feature = "statistics"))]
                let (inline_version_count, out_of_line_version_count) = (None::<u32>, None::<u32>);
                info!(log, "Finished fetching mods"; "inline_version_count" => inline_version_count, "out_of_line_version_count" => out_of_line_version_count);

                Ok::<_, anyhow::Error>((None, ()))
            })
            .await
            .map_err(Into::into)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum SortColumn {
    Relevance,
    Name,
    Owner,
    Downloads,
    Size,
}

impl SortColumn {
    pub const VALUES: &[Self] = &[
        Self::Relevance,
        Self::Name,
        Self::Owner,
        Self::Downloads,
        Self::Size,
    ];

    pub const VALUE_COUNT: usize = Self::VALUES.len();
}

pub type ModIndexReadGuard = RwLockReadGuard<'static, MemoryModIndex>;

pub async fn read_mod_index(game: &str) -> Result<ModIndexReadGuard> {
    let game = *games_by_id()?.get(game).context("No such game")?;
    Ok(MOD_INDEXES
        .get(&*game.thunderstore_url)
        .unwrap()
        .data
        .read()
        .await)
}

pub fn count_mod_index<'a>(mod_index: &'a ModIndexReadGuard, query: &str) -> Result<usize> {
    let log = slog_scope::logger();

    trace!(log, "Counting mods in mod index");

    let start = Instant::now();

    // FIXME: this is wasteful
    let bump = Bump::new();
    let query = Needle {
        needle: query,
        query: Query::new(&bump, query),
    };

    let mut bump = Bump::new();

    let count = mod_index
        .chunks
        .iter()
        .map(|mi| {
            mi.mods()
                .iter()
                .filter_map(|m| {
                    bump.reset();
                    score_mod(&log, &bump, &query, m)
                })
                .filter(|&(_, score)| search::should_include(score))
                .count()
        })
        .sum();

    let elapsed_counting = Instant::now() - start;

    debug!(log, "Counted mods in mod index ({:?})", elapsed_counting);

    Ok(count)
}

/// `sort` must not include the same [`SortColumn`] more than once.
pub fn query_mod_index<'a>(
    mod_index: &'a ModIndexReadGuard,
    query: &str,
    sort: &[SortOption<SortColumn>],
) -> Result<Vec<(&'a ArchivedModRef<'a>, Score)>> {
    let log = slog_scope::logger();

    trace!(log, "Querying mod index");

    let start = Instant::now();

    // FIXME: this is wasteful
    let bump = Bump::new();
    let query = Needle {
        needle: query,
        query: Query::new(&bump, query),
    };

    let mut bump = Bump::new();

    let mut buf = Vec::new();

    for (i, mi) in mod_index.chunks.iter().enumerate() {
        buf.extend(
            mi.mods()
                .iter()
                .filter_map(|m| {
                    bump.reset();
                    score_mod(&log, &bump, &query, m)
                })
                .filter(|&(_, score)| search::should_include(score)),
        );
        if i == 0 {
            buf.reserve(buf.len() * (mod_index.len() - 1));
        }
    }

    let now = Instant::now();
    let elapsed_collecting = now - start;
    let start = now;

    if !sort.is_empty() {
        buf.sort_unstable_by(|&(m1, score1), &(m2, score2)| {
            let mut ordering = std::cmp::Ordering::Equal;
            for &SortOption { column, descending } in sort {
                ordering = match column {
                    SortColumn::Relevance => score1.cmp(&score2),
                    SortColumn::Name => m1.name.cmp(&m2.name),
                    SortColumn::Owner => m1.owner.cmp(&m2.owner),
                    SortColumn::Downloads => m1.total_downloads.cmp(&m2.total_downloads),
                    SortColumn::Size => {
                        let latest_size =
                            |m: &ArchivedModRef| m.versions.first().map(|v| v.file_size);
                        latest_size(m1).cmp(&latest_size(m2))
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

    let elapsed_sorting = Instant::now() - start;

    debug!(
        log,
        "Queried mod index ({:?} collecting, {:?} sorting)", elapsed_collecting, elapsed_sorting
    );

    Ok(buf)
}

fn score_mod<'a, 'b>(
    _log: &slog::Logger,
    bump: &Bump,
    query: &Needle,
    m: &'a ArchivedModRef<'b>,
) -> Option<(&'a ArchivedModRef<'b>, Score)> {
    // Please have mercy for this naming.
    if query.query.query.is_empty() {
        Some((m, Score::MAX))
    } else {
        let owner_score =
            search::score(bump, query, &m.owner).map(|s| std::cmp::max(s / 128, Score::ZERO));
        let name_score = search::score(bump, query, &m.name);
        let score = search::add_scores(name_score, owner_score)?;
        let boosted_score = score
            * m.total_downloads
                .to_native()
                .checked_ilog10()
                .unwrap_or(1)
                .max(1);
        Some((m, boosted_score))
    }
}

pub async fn get_from_mod_index<'a>(
    mod_index: &'a ModIndexReadGuard,
    mod_ids: &[ModId<'_>],
) -> Result<Vec<Option<&'a ArchivedModRef<'a>>>> {
    let log = slog_scope::logger();

    debug!(log, "Getting set of mods from mod index");

    // We need to check potentially tens of thousands of mods, and we don't
    // want O(n*m) complexity. Instead, create an efficient mapping from mod id
    // to index in the results array.
    let mod_ids_idx = mod_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect::<std::collections::HashMap<_, _>>();

    let mut results = vec![None; mod_ids.len()];

    mod_index
        .chunks
        .iter()
        .flat_map(|mi| mi.mods().iter())
        .for_each(|m| {
            if let Some(&i) = mod_ids_idx.get(&ModId {
                owner: InternedString(&*m.owner),
                name: InternedString(&*m.name),
            }) {
                results[i] = Some(m);
            }
        });

    Ok(results)
}

pub async fn get_one_from_mod_index<'a>(
    mod_index: &'a ModIndexReadGuard,
    mod_id: ModId<'_>,
) -> Result<Option<&'a ArchivedModRef<'a>>> {
    let log = slog_scope::logger();

    debug!(log, "Getting one mod from mod index");

    let m = mod_index.chunks.iter().find_map(|mi| {
        mi.mods().iter().find(|m| {
            mod_id
                == ModId {
                    owner: InternedString(&*m.owner),
                    name: InternedString(&*m.name),
                }
        })
    });

    Ok(m)
}

#[cfg(test)]
mod tests {
    use manderrow_types::mods::ArchivedModRef;

    use crate::{
        mod_index::ModIndexReadGuard,
        util::search::{Score, SortOption},
        Reqwest,
    };

    #[test]
    fn mod_index_fetching() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("unable to build tokio runtime")
            .block_on(async {
                let reqwest = Reqwest(reqwest::Client::new());
                super::fetch_mod_index(None, &reqwest, "lethal-company", true, None)
                    .await
                    .unwrap();

                let mod_index = super::read_mod_index("lethal-company").await.unwrap();

                let mod_count = super::count_mod_index(&mod_index, "").unwrap();
                assert!(
                    mod_count >= 40_000,
                    "mod count is lower than expected: {}",
                    mod_count
                );

                let mods = super::query_mod_index(&mod_index, "", &[]).unwrap();
                assert_eq!(mods.len(), mod_count);
            });
    }

    #[test]
    fn mod_index_querying_relevance() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("unable to build tokio runtime")
            .block_on(async {
                let reqwest = Reqwest(reqwest::Client::new());
                super::fetch_mod_index(None, &reqwest, "lethal-company", true, None)
                    .await
                    .unwrap();

                let mod_index = super::read_mod_index("lethal-company").await.unwrap();

                assert_top_result(
                    &mod_index,
                    "more",
                    &[
                        // it would be ideal if these were swapped
                        ("2wheelsNcoffee", "moresuits_2WC"),
                        ("notnotnotswipez", "MoreCompany"),
                    ],
                );
                assert_top_result(
                    &mod_index,
                    "com",
                    &[
                        ("HHunter", "company_cruiser_steering_fix"),
                        // this should certainly not be ranked this high
                        ("Xaymar", "common"),
                        ("notnotnotswipez", "MoreCompany"),
                    ],
                );
            });
    }

    async fn assert_top_result(
        mod_index: &ModIndexReadGuard,
        query: &str,
        top_expected: &[(&str, &str)],
    ) {
        let mod_count = super::count_mod_index(&mod_index, query).unwrap();
        assert!(
            mod_count >= top_expected.len(),
            "mod count is lower than expected: {}",
            mod_count
        );

        let mods = super::query_mod_index(
            &mod_index,
            query,
            &[SortOption {
                column: super::SortColumn::Relevance,
                descending: true,
            }],
        )
        .unwrap();
        assert_eq!(mods.len(), mod_count);
        for ((m, _), &id) in mods.iter().zip(top_expected) {
            assert_eq!((&*m.owner, &*m.name), id, "{}", TopResults(&mods));
        }
    }

    struct TopResults<'a, 'b, 'c>(&'a [(&'b ArchivedModRef<'c>, Score)]);

    impl std::fmt::Display for TopResults<'_, '_, '_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // this is gross, but we're in a test. Who cares.
            let score_width = self.0[0].1.to_string().len();
            write!(
                f,
                "the top results are: {:#?}",
                self.0
                    .iter()
                    .map(|(m, s)| std::fmt::from_fn(move |f| {
                        write!(
                            f,
                            "{: >score_width$}: {}-{}",
                            s,
                            &*m.owner,
                            &*m.name,
                            score_width = score_width
                        )
                    }))
                    .take(20)
                    .collect::<Vec<_>>()
            )
        }
    }
}
