mod memory;

use std::{
    collections::{HashMap, HashSet},
    io::Read as _,
    sync::{atomic::AtomicU64, LazyLock},
};

use anyhow::{Context as _, Result};
use drop_guard::ext::tokio1::JoinHandleExt;
use flate2::bufread::GzDecoder;
use slog::debug;
use tauri::{ipc::Channel, AppHandle, Manager};
use tokio::sync::{Mutex, RwLock};

use crate::{
    games::{GAMES, GAMES_BY_ID},
    http::{fetch_as_blocking, StreamReadable},
    mods::{ArchivedMod, Mod, ModRef, ModVersionRef},
    CommandError, Reqwest,
};

use memory::MemoryModIndex;

#[derive(Default)]
struct ModIndex {
    data: RwLock<Vec<MemoryModIndex>>,
    refresh_lock: Mutex<()>,
    progress: AtomicU64,
    progress_updates: event_listener::Event,
}

impl ModIndex {
    pub fn progress(&self) -> (u32, u32) {
        let v = self.progress.load(std::sync::atomic::Ordering::Acquire);
        (v as u32, (v >> 32) as u32)
    }

    pub fn set_progress(&self, complete: u32, total: u32) {
        self.progress.store(
            (complete as u64) | ((total as u64) << 32),
            std::sync::atomic::Ordering::Release,
        );
        _ = self.progress_updates.notify(usize::MAX);
    }

    pub fn inc_progress(&self) {
        self.progress
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        _ = self.progress_updates.notify(usize::MAX);
    }
}

static MOD_INDEXES: LazyLock<HashMap<&'static str, ModIndex>> = LazyLock::new(|| {
    GAMES
        .iter()
        .map(|game| (&*game.thunderstore_url, ModIndex::default()))
        .collect()
});

async fn fetch_gzipped(reqwest: &Reqwest, url: &str) -> Result<GzDecoder<StreamReadable>> {
    let rdr = fetch_as_blocking(reqwest.get(url)).await?;
    Ok(tokio::task::block_in_place(move || GzDecoder::new(rdr)))
}

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
                event_listener::listener!(mod_index.progress_updates => listener);

                let (completed, total) = mod_index.progress();
                _ = on_event.send(FetchEvent::Progress { completed, total });

                listener.await;

                let (completed, total) = mod_index.progress();
                _ = on_event.send(FetchEvent::Progress { completed, total });
            }
        })
        .abort_on_drop();

        let Ok(_lock) = mod_index.refresh_lock.try_lock() else {
            // just wait for the current refetch to complete.
            _ = mod_index.refresh_lock.lock().await;
            return Ok(());
        };

        mod_index.set_progress(0, 1);
        let chunk_urls =
            fetch_gzipped(&*app_handle.state::<Reqwest>(), &game.thunderstore_url).await?;
        let chunk_urls =
            tokio::task::block_in_place(|| serde_json::from_reader::<_, Vec<String>>(chunk_urls))
                .context("Unable to decode chunk URLs from Thunderstore")?;
        mod_index.set_progress(
            1,
            chunk_urls
                .len()
                .checked_add(1)
                .and_then(|n| n.try_into().ok())
                .context("too many chunk urls")?,
        );

        let new_mod_index =
            futures::future::try_join_all(chunk_urls.into_iter().map(|url| async {
                let log = log.clone();
                let app_handle = app_handle.clone();
                tokio::task::spawn(async move {
                    let mut buf = Vec::new();
                    let mut rdr = fetch_gzipped(&*app_handle.state::<Reqwest>(), &url).await?;
                    tokio::task::block_in_place(move || {
                        rdr.read_to_end(&mut buf)?;
                        let mods = simd_json::from_slice::<Vec<Mod>>(&mut buf)?;
                        let mods = rkyv::util::with_arena(|arena| {
                            let mut serializer = rkyv_intern::InterningAdapter::new(
                                rkyv::ser::Serializer::new(
                                    rkyv::util::AlignedVec::<16>::new(),
                                    arena.acquire(),
                                    rkyv::ser::sharing::Share::new(),
                                ),
                                rkyv_intern::Interner::<String>::default(),
                            );
                            rkyv::api::serialize_using::<_, rkyv::rancor::Error>(
                                &mods,
                                &mut serializer,
                            )?;
                            Ok::<_, rkyv::rancor::Error>(serializer.into_serializer().into_writer())
                        })?;
                        debug!(
                            log,
                            "{} bytes of JSON -> {} bytes in memory",
                            buf.len(),
                            mods.len()
                        );
                        let index = MemoryModIndex::new(mods, |data| {
                            rkyv::access::<_, rkyv::rancor::Error>(data)
                        })?;
                        mod_index.inc_progress();
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

// TODO: use register_asynchronous_uri_scheme_protocol to stream the json back without buffering
#[tauri::command]
pub async fn query_mod_index(
    game: &str,
    query: &str,
    sort: Vec<SortOption>,
    skip: Option<usize>,
    limit: Option<usize>,
    exact: Option<HashSet<&str>>,
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
                        exact.contains(m.full_name.as_str())
                    } else {
                        true
                    }
                })
                .filter_map(|m| {
                    if query.is_empty() {
                        Some((m, 0.0))
                    } else {
                        rff::match_and_score(&query, &m.full_name).map(|(_, score)| (m, score))
                    }
                })
        })
        .collect::<Vec<_>>();
    if !sort.is_empty() {
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
    }

    let count = buf.len();

    fn map_to_json<'a>(
        it: impl IntoIterator<Item = (&'a ArchivedMod, f64)>,
    ) -> Vec<simd_json::OwnedValue> {
        it.into_iter()
            .map(|(m, _)| {
                simd_json::serde::to_owned_value(ModRef {
                    name: &m.name,
                    full_name: &m.full_name,
                    owner: &m.owner,
                    package_url: m.package_url.as_deref(),
                    donation_link: m.donation_link.as_deref(),
                    date_created: &m.date_created,
                    date_updated: &m.date_updated,
                    rating_score: m.rating_score.into(),
                    is_pinned: m.is_pinned,
                    is_deprecated: m.is_deprecated,
                    has_nsfw_content: m.has_nsfw_content,
                    categories: m.categories.iter().map(|s| s.0.as_str()).collect(),
                    versions: m
                        .versions
                        .iter()
                        .map(|v| ModVersionRef {
                            name: &v.name,
                            full_name: &v.full_name,
                            description: &v.description,
                            icon: &v.icon,
                            version_number: &v.version_number,
                            dependencies: v.dependencies.iter().map(|s| s.0.as_str()).collect(),
                            download_url: &v.download_url,
                            downloads: v.downloads.into(),
                            date_created: &v.date_created,
                            website_url: v.website_url.as_deref(),
                            is_active: v.is_active.into(),
                            uuid4: v.uuid4,
                            file_size: v.file_size.into(),
                        })
                        .collect(),
                    uuid4: m.uuid4,
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
