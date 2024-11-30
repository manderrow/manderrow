use std::{
    collections::HashMap,
    io::{Cursor, Read as _},
    ptr::NonNull,
    sync::LazyLock,
};

use bytes::Bytes;
use flate2::bufread::GzDecoder;
use games::Game;
use parking_lot::RwLock;
use uuid::Uuid;

pub mod games;

#[derive(Debug, Clone, serde::Serialize)]
struct Error {
    message: String,
    backtrace: String,
}

impl<T: std::fmt::Display> From<T> for Error {
    #[track_caller]
    fn from(value: T) -> Self {
        let backtrace = std::backtrace::Backtrace::force_capture();
        println!("{value}\nBacktrace:\n{backtrace}");
        Self {
            message: value.to_string(),
            backtrace: backtrace.to_string(),
        }
    }
}

struct ModIndex {
    data: NonNull<[u8]>,
    mods: Vec<Mod<'static>>,
}

impl ModIndex {
    pub fn mods(&self) -> &Vec<Mod<'_>> {
        &self.mods
    }
}

unsafe impl Send for ModIndex {}
unsafe impl Sync for ModIndex {}

impl Drop for ModIndex {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(self.data.as_ptr()) });
    }
}

static GAMES: LazyLock<Vec<Game>> =
    LazyLock::new(|| serde_json::from_str(include_str!("games.json")).unwrap());

static GAMES_BY_ID: LazyLock<HashMap<&'static str, &'static Game>> =
    LazyLock::new(|| GAMES.iter().map(|g| (&*g.id, g)).collect());

static MOD_INDEXES: LazyLock<HashMap<&'static str, RwLock<Vec<ModIndex>>>> = LazyLock::new(|| {
    GAMES
        .iter()
        .map(|game| (&*game.thunderstore_url, RwLock::default()))
        .collect()
});

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Mod<'a> {
    name: &'a str,
    full_name: &'a str,
    owner: &'a str,
    #[serde(default)]
    package_url: Option<&'a str>,
    donation_link: Option<&'a str>,
    date_created: &'a str,
    date_updated: &'a str,
    rating_score: u32,
    is_pinned: bool,
    is_deprecated: bool,
    has_nsfw_content: bool,
    categories: Vec<&'a str>,
    versions: Vec<ModVersion<'a>>,
    uuid4: Uuid,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct ModVersion<'a> {
    name: &'a str,
    full_name: &'a str,
    description: &'a str,
    icon: &'a str,
    version_number: &'a str,
    dependencies: Vec<&'a str>,
    download_url: &'a str,
    downloads: u64,
    date_created: &'a str,
    website_url: Option<&'a str>,
    is_active: bool,
    uuid4: Uuid,
    file_size: u64,
}

async fn fetch_gzipped(url: &str) -> Result<GzDecoder<Cursor<Bytes>>, Error> {
    let bytes = tauri_plugin_http::reqwest::get(url).await?.bytes().await?;
    Ok(GzDecoder::new(Cursor::new(bytes)))
}

#[tauri::command]
async fn get_games() -> &'static [Game] {
    &*GAMES
}

#[tauri::command]
async fn fetch_mod_index(game: &str, refresh: bool) -> Result<(), Error> {
    let game = *GAMES_BY_ID.get(game).ok_or("No such game")?;
    let mod_index = MOD_INDEXES.get(&*game.thunderstore_url).unwrap();

    if refresh || mod_index.read().is_empty() {
        let chunk_urls = serde_json::from_reader::<_, Vec<String>>(fetch_gzipped(&game.thunderstore_url).await?)?;

        let new_mod_index =
            futures::future::try_join_all(chunk_urls.into_iter().map(|url| async {
                tokio::task::spawn(async move {
                    let mut rdr = fetch_gzipped(&url).await?;
                    tokio::task::block_in_place(|| {
                        let mut buf = Vec::new();
                        rdr.read_to_end(&mut buf)?;
                        let mut index = ModIndex {
                            data: NonNull::new(Box::into_raw(buf.into_boxed_slice())).unwrap(),
                            mods: Vec::new(),
                        };
                        index.mods =
                            simd_json::from_slice::<Vec<Mod>>(unsafe { index.data.as_mut() })?;
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

#[derive(serde::Serialize)]
struct QueryResult<'a> {
    mods: Vec<&'a Mod<'a>>,
    count: usize,
}

#[tauri::command]
fn query_mod_index(game: &str, query: &str) -> Result<simd_json::OwnedValue, Error> {
    let game = *GAMES_BY_ID.get(game).ok_or("No such game")?;
    let mod_index = MOD_INDEXES.get(&*game.thunderstore_url).unwrap().read();

    let mut buf = mod_index
        .iter()
        .flat_map(|mi| {
            mi.mods().iter().filter_map(|m| {
                rff::match_and_score(&query, &m.full_name).map(|(_, score)| (m, score))
            })
        })
        .collect::<Vec<_>>();
    buf.sort_unstable_by(|(m1, score1), (m2, score2)| {
        score1
            .total_cmp(score2)
            .reverse()
            .then_with(|| match (&*m1.versions, &*m2.versions) {
                ([ModVersion { downloads: a, .. }, ..], [ModVersion { downloads: b, .. }, ..]) => {
                    a.cmp(b).reverse()
                }
                _ => std::cmp::Ordering::Equal,
            })
            .then_with(|| m1.full_name.cmp(&m2.full_name))
    });

    let count = buf.len();

    buf.truncate(50);

    let buf = buf.into_iter().map(|(m, _)| m).collect::<Vec<_>>();

    Ok(simd_json::serde::to_owned_value(QueryResult {
        count,
        mods: buf,
    })?)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_games,
            fetch_mod_index,
            query_mod_index
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
