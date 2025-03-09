pub mod commands;

use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

pub static GAMES: LazyLock<Vec<Game>> =
    LazyLock::new(|| serde_json::from_str(include_str!("games.json")).unwrap());

pub static GAMES_BY_ID: LazyLock<HashMap<&'static str, &'static Game>> =
    LazyLock::new(|| GAMES.iter().map(|g| (&*g.id, g)).collect());

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Game<'a> {
    /// Unique internal id for the game.
    #[serde(borrow)]
    pub id: Cow<'a, str>,
    /// Display name of the game.
    #[serde(alias = "displayName", borrow)]
    pub name: Cow<'a, str>,
    /// URL of the Thunderstore mod index for the game.
    #[serde(alias = "thunderstoreUrl", borrow)]
    pub thunderstore_url: Cow<'a, str>,
    #[serde(alias = "steamFolderName", borrow)]
    pub steam_folder_name: Cow<'a, str>,
    #[serde(alias = "exeName", borrow)]
    pub exe_names: Vec<Cow<'a, str>>,
    #[serde(alias = "dataFolderName", borrow)]
    pub data_folder_name: Cow<'a, str>,
    #[serde(alias = "storePlatformMetadata")]
    #[serde(borrow)]
    pub store_platform_metadata: Vec<StorePlatformMetadata<'a>>,
    #[serde(alias = "instanceType")]
    pub instance_type: InstanceType,
    #[serde(alias = "packageLoader")]
    pub package_loader: PackageLoader,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "storePlatform")]
pub enum StorePlatformMetadata<'a> {
    Steam {
        #[serde(alias = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    SteamDirect {
        #[serde(alias = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    #[serde(alias = "Epic Games Store")]
    Epic {
        #[serde(alias = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    #[serde(alias = "Xbox Game Pass")]
    Xbox {
        #[serde(alias = "storeIdentifier", borrow)]
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
