use std::{collections::HashMap, io::Cursor, sync::LazyLock};

use bytes::Bytes;
use flate2::bufread::GzDecoder;
use parking_lot::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Game {
    /// Unique internal id for the game.
    id: &'static str,
    /// Display name of the game.
    name: &'static str,
    /// Id of the Thunderstore mod index for the game. May not be unique.
    thunderstore_id: &'static str,
    // TODO: other fields (icon, steam id, etc.)
}

const GAMES: &[Game] = &[
    Game {
        id: "riskofrain2",
        name: "Risk of Rain 2",
        thunderstore_id: "riskofrain2",
    },
    Game {
        id: "lethal-company",
        name: "Lethal Company",
        thunderstore_id: "lethal-company",
    },
];

static MOD_INDEX: LazyLock<HashMap<&'static str, RwLock<Vec<Mod>>>> = LazyLock::new(|| {
    GAMES
        .iter()
        .map(|game| (game.id, RwLock::new(Vec::new())))
        .collect()
});

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Mod {
    name: String,
    full_name: String,
    owner: String,
    #[serde(default)]
    package_url: Option<String>,
    donation_link: Option<String>,
    date_created: String,
    date_updated: String,
    rating_score: u32,
    is_pinned: bool,
    is_deprecated: bool,
    has_nsfw_content: bool,
    categories: Vec<String>,
    versions: Vec<ModVersion>,
    uuid4: Uuid,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct ModVersion {
    name: String,
    full_name: String,
    description: String,
    icon: String,
    version_number: String,
    dependencies: Vec<String>,
    download_url: String,
    downloads: u64,
    date_created: String,
    website_url: Option<String>,
    is_active: bool,
    uuid4: Uuid,
    file_size: u64,
}

async fn fetch_gzipped(url: &str) -> Result<GzDecoder<Cursor<Bytes>>, String> {
    let bytes = tauri_plugin_http::reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    Ok(GzDecoder::new(Cursor::new(bytes)))
}

#[tauri::command]
async fn get_games() -> &'static [Game] {
    GAMES
}

#[tauri::command]
async fn fetch_mod_index(game: &str) -> Result<(), String> {
    let result = (|| async move {
        let chunk_urls = serde_json::from_reader::<_, Vec<String>>(
            fetch_gzipped(&format!(
                "https://thunderstore.io/c/{game}/api/v1/package-listing-index/"
            ))
            .await?,
        )
        .map_err(|e| e.to_string())?;
        let mod_index = MOD_INDEX
            .get(game)
            .ok_or_else(|| "No such game".to_owned())?;
        let new_mod_index =
            futures::future::try_join_all(chunk_urls.into_iter().map(|url| async move {
                tokio::task::spawn(async move {
                    let rdr = fetch_gzipped(&url).await?;
                    tokio::task::block_in_place(|| {
                        serde_json::from_reader::<_, Vec<Mod>>(rdr).map_err(|e| e.to_string())
                    })
                })
                .await
                .map_err(|e| e.to_string())?
            }))
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        *mod_index.write() = new_mod_index;
        Ok(())
    })()
    .await;
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("{e}");
            Err(e)
        }
    }
}

#[derive(serde::Serialize)]
struct QueryResult {
    mods: Vec<Mod>,
    count: usize,
}

#[tauri::command]
async fn query_mod_index(game: &str, query: String) -> Result<QueryResult, &'static str> {
    let mod_index = MOD_INDEX.get(game).ok_or("No such game")?.read();
    if query.is_empty() {
        return Ok(QueryResult {
            mods: mod_index.iter().take(50).cloned().collect(),
            count: mod_index.len(),
        });
    }
    let mut buf = mod_index
        .iter()
        .filter_map(|m| rff::match_and_score(&query, &m.full_name).map(|(_, score)| (m, score)))
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
    let mods = buf
        .into_iter()
        .take(50)
        .map(|(m, _)| m)
        .cloned()
        .collect::<Vec<_>>();
    Ok(QueryResult { mods, count })
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
