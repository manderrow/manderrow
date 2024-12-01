use std::{
    collections::HashMap,
    io::{BufRead as _, Read as _},
    pin::pin,
    sync::LazyLock,
};

use bytes::{Buf as _, Bytes};
use flate2::bufread::GzDecoder;
use futures::StreamExt as _;
use parking_lot::RwLock;

use crate::{
    games::{GAMES, GAMES_BY_ID},
    mod_index::ModIndex,
    mods::{Mod, ModVersion},
    Error,
};

static MOD_INDEXES: LazyLock<HashMap<&'static str, RwLock<Vec<ModIndex>>>> = LazyLock::new(|| {
    GAMES
        .iter()
        .map(|game| (&*game.thunderstore_url, RwLock::default()))
        .collect()
});

async fn fetch_gzipped(url: &str) -> Result<GzDecoder<StreamReadable>, Error> {
    let resp = tauri_plugin_http::reqwest::get(url).await?;
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    tokio::task::spawn(async move {
        let mut stream = pin!(resp.bytes_stream());
        while let Some(r) = stream.next().await {
            if tx
                .send(r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
                .await
                .is_err()
            {
                break;
            }
        }
    });
    Ok(tokio::task::block_in_place(move || {
        GzDecoder::new(StreamReadable::new(rx))
    }))
}

struct StreamReadable {
    rx: tokio::sync::mpsc::Receiver<Result<Bytes, std::io::Error>>,
    bytes: Bytes,
}

impl StreamReadable {
    pub fn new(rx: tokio::sync::mpsc::Receiver<Result<Bytes, std::io::Error>>) -> Self {
        Self {
            rx,
            bytes: Bytes::new(),
        }
    }
}

impl std::io::Read for StreamReadable {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.fill_buf()?.is_empty() {
            return Ok(0);
        }

        let bytes = &mut self.bytes;

        // copy_to_slice requires the bytes to have enough remaining bytes
        // to fill buf.
        let n = buf.len().min(bytes.remaining());

        // <Bytes as Buf>::copy_to_slice copies and consumes the bytes
        bytes.copy_to_slice(&mut buf[..n]);

        Ok(n)
    }
}

impl std::io::BufRead for StreamReadable {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        let bytes = &mut self.bytes;

        if !bytes.has_remaining() {
            if let Some(new_bytes) = self.rx.blocking_recv() {
                // new_bytes are guaranteed to be non-empty.
                *bytes = new_bytes?;
            }
        }

        Ok(&*bytes)
    }

    fn consume(&mut self, amt: usize) {
        self.bytes.advance(amt);
    }
}

#[tauri::command]
pub async fn fetch_mod_index(game: &str, refresh: bool) -> Result<(), Error> {
    let game = *GAMES_BY_ID.get(game).ok_or("No such game")?;
    let mod_index = MOD_INDEXES.get(&*game.thunderstore_url).unwrap();

    if refresh || mod_index.read().is_empty() {
        let chunk_urls = fetch_gzipped(&game.thunderstore_url).await?;
        let chunk_urls =
            tokio::task::block_in_place(|| serde_json::from_reader::<_, Vec<String>>(chunk_urls))?;

        let new_mod_index =
            futures::future::try_join_all(chunk_urls.into_iter().map(|url| async {
                tokio::task::spawn(async move {
                    let mut buf = Vec::new();
                    let mut rdr = fetch_gzipped(&url).await?;
                    tokio::task::block_in_place(move || {
                        rdr.read_to_end(&mut buf)?;
                        let index = ModIndex::new(buf.into_boxed_slice(), |data| {
                            simd_json::from_slice::<Vec<Mod>>(data)
                        })?;
                        Ok::<_, Error>(index)
                    })
                })
                .await?
            }))
            .await?;
        *mod_index.write() = new_mod_index;
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

#[derive(serde::Serialize)]
pub struct QueryResult<'a> {
    mods: Vec<&'a Mod<'a>>,
    count: usize,
}

#[tauri::command]
pub fn query_mod_index(
    game: &str,
    query: &str,
    sort: Vec<SortOption>,
) -> Result<simd_json::OwnedValue, Error> {
    let game = *GAMES_BY_ID.get(game).ok_or("No such game")?;
    let mod_index = MOD_INDEXES.get(&*game.thunderstore_url).unwrap().read();

    let mut buf = mod_index
        .iter()
        .flat_map(|mi| {
            mi.mods().iter().filter_map(|m| {
                if query.is_empty() {
                    Some((m, 0.0))
                } else {
                    rff::match_and_score(&query, &m.full_name).map(|(_, score)| (m, score))
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
                SortColumn::Downloads => match (&*m1.versions, &*m2.versions) {
                    (
                        [ModVersion { downloads: a, .. }, ..],
                        [ModVersion { downloads: b, .. }, ..],
                    ) => a.cmp(b),
                    _ => std::cmp::Ordering::Equal,
                },
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

    buf.truncate(50);

    let buf = buf.into_iter().map(|(m, _)| m).collect::<Vec<_>>();

    Ok(simd_json::serde::to_owned_value(QueryResult {
        count,
        mods: buf,
    })?)
}
