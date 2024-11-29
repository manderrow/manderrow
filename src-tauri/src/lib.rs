use std::{
    borrow::Cow,
    io::{Cursor, Read as _},
    sync::{Arc, LazyLock},
};

use bytes::Bytes;
use flate2::bufread::GzDecoder;
use serde_with::serde_as;
use uuid::Uuid;

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

static DATABASE: LazyLock<sqlite::ConnectionThreadSafe> = LazyLock::new(|| {
    unsafe { assert_eq!(sqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(sqlite::ffi::sqlite3_spellfix_init as unsafe extern "C" fn(*mut sqlite3_sys::sqlite3, *mut *mut std::ffi::c_char, *const sqlite3_sys::sqlite3_api_routines) -> std::ffi::c_int))), 0, "Could not load extension") };
    let conn = sqlite::Connection::open_thread_safe(":memory:").unwrap();
    // unsafe { assert_eq!(sqlite::ffi::sqlite3_enable_load_extension(conn.as_raw(), 1), 0, "Could not enable extension loading") };
    conn.execute("CREATE VIRTUAL TABLE mods_fts5 USING fts5(full_name)")
        .unwrap();
    conn.execute("CREATE VIRTUAL TABLE mods_fts5_v_col USING fts5vocab(mods_fts5, col)")
        .unwrap();
    conn.execute("CREATE VIRTUAL TABLE mods_spellfix1 USING spellfix1")
        .unwrap();
    conn.execute(
        "CREATE TABLE mods (
        game TEXT NOT NULL,
        name TEXT NOT NULL,
        full_name TEXT NOT NULL,
        owner TEXT NOT NULL,
        data BLOB NOT NULL,
        fts5_id INTEGER NOT NULL,
        PRIMARY KEY (game, full_name),
        FOREIGN KEY (fts5_id) REFERENCES mods_fts5 (rowid)
    )",
    )
    .unwrap();
    conn
});

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
#[serde_as]
struct Mod<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    full_name: Cow<'a, str>,
    #[serde(borrow)]
    owner: Cow<'a, str>,
    #[serde(default)]
    #[serde_as(as = "Option<BorrowCow>")]
    package_url: Option<Cow<'a, str>>,
    #[serde_as(as = "Option<BorrowCow>")]
    donation_link: Option<Cow<'a, str>>,
    #[serde(borrow)]
    date_created: Cow<'a, str>,
    #[serde(borrow)]
    date_updated: Cow<'a, str>,
    rating_score: u32,
    is_pinned: bool,
    is_deprecated: bool,
    has_nsfw_content: bool,
    #[serde(borrow)]
    categories: Vec<Cow<'a, str>>,
    #[serde(borrow)]
    versions: Vec<ModVersion<'a>>,
    uuid4: Uuid,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
#[serde_as]
struct ModVersion<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    full_name: Cow<'a, str>,
    #[serde(borrow)]
    description: Cow<'a, str>,
    #[serde(borrow)]
    icon: Cow<'a, str>,
    #[serde(borrow)]
    version_number: Cow<'a, str>,
    dependencies: Vec<Cow<'a, str>>,
    #[serde(borrow)]
    download_url: Cow<'a, str>,
    downloads: u64,
    #[serde(borrow)]
    date_created: Cow<'a, str>,
    #[serde_as(as = "Option<BorrowCow>")]
    website_url: Option<Cow<'a, str>>,
    is_active: bool,
    uuid4: Uuid,
    file_size: u64,
}

async fn fetch_gzipped(url: &str) -> Result<GzDecoder<Cursor<Bytes>>, Error> {
    let bytes = tauri_plugin_http::reqwest::get(url)
        .await?
        .bytes()
        .await?;
    Ok(GzDecoder::new(Cursor::new(bytes)))
}

#[tauri::command]
async fn get_games() -> &'static [Game] {
    GAMES
}

#[tauri::command]
async fn fetch_mod_index(game: String) -> Result<(), Error> {
    let game = Arc::new(game);
    let chunk_urls = serde_json::from_reader::<_, Vec<String>>(
        fetch_gzipped(&format!(
            "https://thunderstore.io/c/{game}/api/v1/package-listing-index/"
        ))
        .await?,
    )?;

    { // clear the index for the game
        let mut stmt = DATABASE.prepare("DELETE FROM mods WHERE game = :game")?;
        stmt.bind((":game", &**game))?;
        while matches!(stmt.next()?, sqlite::State::Row) {}

        let mut stmt = DATABASE.prepare("DELETE FROM mods_fts5 WHERE rowid IN ( SELECT fts5_id FROM mods WHERE game = :game )")?;
        stmt.bind((":game", &**game))?;
        while matches!(stmt.next()?, sqlite::State::Row) {}
    }

    futures::future::try_join_all(chunk_urls.into_iter().map(|url| async {
        let game = game.clone();
            tokio::task::spawn(async move {
                let mut rdr = fetch_gzipped(&url).await?;
                tokio::task::block_in_place(|| {
                    let mut buf = Vec::new();
                    rdr.read_to_end(&mut buf)?;
                    use serde::Deserializer as _;
                    struct Visitor<'a> {
                        game: &'a str,
                    }
                    impl<'a, 'de> serde::de::Visitor<'de> for Visitor<'a> {
                        type Value = ();

                        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                            f.write_str("")
                        }

                        fn visit_seq<A: serde::de::SeqAccess<'de> >(self, mut access: A) -> Result<Self::Value, A::Error> {
                            while let Some(m) = access.next_element::<Mod>()? {
                                let mut stmt = DATABASE.prepare("INSERT INTO mods_fts5 (full_name) VALUES (:full_name) RETURNING rowid").map_err(<A::Error as serde::de::Error>::custom)?;
                                stmt.bind((":full_name", &*m.full_name)).map_err(<A::Error as serde::de::Error>::custom)?;
                                stmt.next().map_err(<A::Error as serde::de::Error>::custom)?;
                                let fts5_id = stmt.read::<i64, _>(0).map_err(<A::Error as serde::de::Error>::custom)?;

                                let mut stmt = DATABASE.prepare("INSERT INTO mods (game, name, full_name, owner, data, fts5_id) VALUES (:game, :name, :full_name, :owner, :data, :fts5_id)").map_err(<A::Error as serde::de::Error>::custom)?;
                                stmt.bind((":game", self.game)).map_err(<A::Error as serde::de::Error>::custom)?;
                                stmt.bind((":name", &*m.name)).map_err(<A::Error as serde::de::Error>::custom)?;
                                stmt.bind((":full_name", &*m.full_name)).map_err(<A::Error as serde::de::Error>::custom)?;
                                stmt.bind((":owner", &*m.owner)).map_err(<A::Error as serde::de::Error>::custom)?;
                                stmt.bind((":data", &*serde_json::to_vec(&m).map_err(<A::Error as serde::de::Error>::custom)?)).map_err(<A::Error as serde::de::Error>::custom)?;
                                stmt.bind((":fts5_id", fts5_id)).map_err(<A::Error as serde::de::Error>::custom)?;
                                while matches!(stmt.next().map_err(<A::Error as serde::de::Error>::custom)?, sqlite::State::Row) {}
                            }
                            Ok(())
                        }
                    }
                    serde_json::Deserializer::from_slice(&buf)
                    // serde_json::Deserializer::from_reader(rdr)
                        .deserialize_seq(Visitor {game: &**game})?;
                    Ok::<_, Error>(())
                })
            })
            .await?
        }))
        .await?;

    {
        let mut stmt = DATABASE.prepare("INSERT INTO mods_spellfix1(word) SELECT term FROM mods_fts5_v_col WHERE col='*'")?;
        while matches!(stmt.next()?, sqlite::State::Row) {}
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct QueryResult {
    mods: Vec<Mod<'static>>,
    count: usize,
}

fn clone_owned<T: ?Sized + ToOwned>(c: Cow<T>) -> Cow<'static, T> {
    Cow::Owned(c.into_owned())
}

#[tauri::command]
fn query_mod_index(game: &str, query: String) -> Result<QueryResult, Error> {
    let mut corrected_query = String::new();

    for term in query.split(|c: char| !c.is_ascii_alphanumeric() && u32::from(c) < 128).filter(|term| !term.is_empty()) {
        let mut stmt = DATABASE.prepare("SELECT word FROM mods_spellfix1 WHERE word MATCH ? and top=1")?;
        stmt.bind((1, term))?;
        let corrected_term_slot: String;
        let corrected_term = if matches!(stmt.next()?, sqlite::State::Row) {
            corrected_term_slot = stmt.read::<String, _>(0)?;
            &corrected_term_slot
        } else {
            term
        };
        if !corrected_query.is_empty() {
            corrected_query.push(' ');
        }
        corrected_query.push_str(corrected_term);
    }

    println!("Querying mods for {game}: {query:?} / {corrected_query:?}");

    let count = {
        let mut stmt = DATABASE
            .prepare(
                r#"SELECT COUNT(full_name) FROM mods WHERE game = :game AND (:query = "" OR fts5_id in ( SELECT rowid FROM mods_fts5(:query) ))"#,
            )?;
        stmt.bind((":game", game))?;
        stmt.bind((":query", &*corrected_query))?;
        stmt.next()?;
        usize::try_from(stmt.read::<i64, _>(0)?)?
    };

    let mods = {
        let mut stmt = DATABASE
            .prepare(
                r#"SELECT data FROM mods WHERE game = :game AND (:query = "" OR fts5_id in ( SELECT rowid FROM mods_fts5(:query) )) LIMIT 50"#,
            )?;
        stmt.bind((":game", game))?;
        stmt.bind((":query", &*corrected_query))?;
        stmt.into_iter()
            .map(|r| {
                let mut row = r?;
                let data = row.take(0);
                let data = Vec::<u8>::try_from(data)?;
                let m = serde_json::from_slice::<Mod>(&data)?;
                Ok(Mod::<'static> {
                    name: clone_owned(m.name),
                    full_name: clone_owned(m.full_name),
                    owner: clone_owned(m.owner),
                    package_url: m.package_url.map(clone_owned),
                    donation_link: m.donation_link.map(clone_owned),
                    date_created: clone_owned(m.date_created),
                    date_updated: clone_owned(m.date_updated),
                    rating_score: m.rating_score,
                    is_pinned: m.is_pinned,
                    is_deprecated: m.is_deprecated,
                    has_nsfw_content: m.has_nsfw_content,
                    categories: m.categories.into_iter().map(clone_owned).collect(),
                    versions: m
                        .versions
                        .into_iter()
                        .map(|v| ModVersion {
                            name: clone_owned(v.name),
                            full_name: clone_owned(v.full_name),
                            description: clone_owned(v.description),
                            icon: clone_owned(v.icon),
                            version_number: clone_owned(v.version_number),
                            dependencies: v.dependencies.into_iter().map(clone_owned).collect(),
                            download_url: clone_owned(v.download_url),
                            downloads: v.downloads,
                            date_created: clone_owned(v.date_created),
                            website_url: v.website_url.map(clone_owned),
                            is_active: v.is_active,
                            uuid4: v.uuid4,
                            file_size: v.file_size,
                        })
                        .collect(),
                    uuid4: m.uuid4.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?
    };
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
