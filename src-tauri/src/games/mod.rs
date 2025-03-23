pub mod commands;

use std::{borrow::Cow, collections::HashMap, marker::PhantomData, sync::LazyLock};

use slog_scope::warn;

pub static GAMES: LazyLock<Vec<Game>> =
    LazyLock::new(|| serde_json::from_str(include_str!("games.json")).unwrap());

struct IndexedGameData<T>(Vec<T>);

impl<'de, T: Clone + Default + serde::Deserialize<'de>> serde::Deserialize<'de>
    for IndexedGameData<T>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<fn() -> T>);
        impl<'de, T: Clone + Default + serde::Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = IndexedGameData<T>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map of game ids to data values")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                use serde::de::Error;
                let mut buf = (0..GAMES.len()).map(|_| None::<T>).collect::<Vec<_>>();
                while let Some(id) = map.next_key::<&str>()? {
                    let value = map.next_value()?;
                    let mut iter = GAMES
                        .iter()
                        .enumerate()
                        .filter(|(_, g)| g.thunderstore_id == id)
                        .map(|(i, _)| i);
                    let Some(found) = iter.next() else {
                        // TODO: make this a hard error
                        //A::Error::invalid_value(serde::de::Unexpected::Str(id), &"a valid game id")
                        warn!("Skipping unused entry for {id:?} in a game data file");
                        continue;
                    };
                    buf[found] = Some(value);
                    for i in iter {
                        let value = buf[found].clone();
                        buf[i] = value;
                    }
                }
                Ok(IndexedGameData(
                    buf.into_iter()
                        .enumerate()
                        .map(|(i, o)| {
                            // TODO: make this a hard error
                            //o.ok_or_else(|| A::Error::missing_field(GAMES[i].id))
                            Ok::<_, A::Error>(match o {
                                Some(t) => t,
                                None => {
                                    warn!(
                                        "Ignoring missing entry for {:?} in a game data file",
                                        GAMES[i].id
                                    );
                                    Default::default()
                                }
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
        }
        deserializer.deserialize_map(Visitor(PhantomData))
    }
}

pub static GAMES_MOD_DOWNLOADS: LazyLock<Vec<u64>> = LazyLock::new(|| {
    serde_json::from_str::<IndexedGameData<_>>(include_str!("gameModDownloads.json"))
        .expect("Failed to load gameModDownloads.json")
        .0
});

pub static GAMES_REVIEWS: LazyLock<Vec<Option<u64>>> = LazyLock::new(|| {
    serde_json::from_str::<IndexedGameData<_>>(include_str!("gameReviews.json"))
        .expect("Failed to load gameReviews.json")
        .0
});

pub static GAMES_BY_ID: LazyLock<HashMap<&'static str, &'static Game>> =
    LazyLock::new(|| GAMES.iter().map(|g| (&*g.id, g)).collect());

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Game<'a> {
    /// Unique internal id for the game.
    pub id: &'a str,
    /// Display name of the game.
    #[serde(borrow)]
    pub name: Cow<'a, str>,
    /// Thunderstore community id for the game.
    #[serde(rename = "thunderstoreId", borrow)]
    pub thunderstore_id: &'a str,
    /// URL of the Thunderstore mod index for the game.
    #[serde(rename = "thunderstoreUrl", borrow)]
    pub thunderstore_url: Cow<'a, str>,
    #[serde(rename = "steamFolderName", borrow)]
    pub steam_folder_name: Cow<'a, str>,
    #[serde(rename = "exeNames", borrow)]
    pub exe_names: Vec<Cow<'a, str>>,
    #[serde(rename = "dataFolderName", borrow)]
    pub data_folder_name: Cow<'a, str>,
    #[serde(rename = "storePlatformMetadata", borrow)]
    pub store_platform_metadata: Vec<StorePlatformMetadata<'a>>,
    #[serde(rename = "instanceType")]
    pub instance_type: InstanceType,
    #[serde(rename = "packageLoader")]
    pub package_loader: PackageLoader,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "storePlatform")]
pub enum StorePlatformMetadata<'a> {
    Steam {
        #[serde(rename = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    SteamDirect {
        #[serde(rename = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    #[serde(alias = "Epic Games Store")]
    Epic {
        #[serde(rename = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    #[serde(alias = "Xbox Game Pass")]
    Xbox {
        #[serde(rename = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    #[serde(alias = "Oculus Store")]
    Oculus,
    #[serde(alias = "Origin / EA Desktop")]
    Origin,
    Other,
}

impl<'a> StorePlatformMetadata<'a> {
    pub fn steam_or_direct(&self) -> Option<&str> {
        match self {
            StorePlatformMetadata::Steam { store_identifier }
            | StorePlatformMetadata::SteamDirect { store_identifier } => Some(store_identifier),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, strum::EnumString)]
pub enum PackageLoader {
    BepInEx,
    MelonLoader,
    NorthStar,
    GodotML,
    AncientDungeonVR,
    ShimLoader,
    Lovely,
    ReturnOfModding,
    GDWeave,
}

impl PackageLoader {
    pub const fn as_str(self) -> &'static str {
        macro_rules! implement {
            ($($variant:ident),*) => {
                match self {
                    $(Self::$variant => stringify!($variant)),*
                }
            };
        }
        implement!(
            BepInEx,
            MelonLoader,
            NorthStar,
            GodotML,
            AncientDungeonVR,
            ShimLoader,
            Lovely,
            ReturnOfModding,
            GDWeave
        )
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum InstanceType {
    Game,
    Server,
}
