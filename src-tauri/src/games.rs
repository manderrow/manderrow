use std::{collections::HashMap, sync::LazyLock};

pub static GAMES: LazyLock<Vec<Game>> =
    LazyLock::new(|| serde_json::from_str(include_str!("games.json")).unwrap());

pub static GAMES_BY_ID: LazyLock<HashMap<&'static str, &'static Game>> =
    LazyLock::new(|| GAMES.iter().map(|g| (&*g.id, g)).collect());

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    /// Unique internal id for the game.
    pub id: String,
    /// Display name of the game.
    pub name: String,
    /// URL of the Thunderstore mod index for the game.
    #[serde(rename = "thunderstoreUrl")]
    pub thunderstore_url: String,
    #[serde(rename = "steamFolderName")]
    pub steam_folder_name: String,
    #[serde(rename = "exeNames")]
    pub exe_names: Vec<String>,
    #[serde(rename = "dataFolderName")]
    pub data_folder_name: String,
    #[serde(rename = "storePlatformMetadata")]
    pub store_platform_metadata: Vec<StorePlatformMetadata>,
    #[serde(rename = "instanceType")]
    pub instance_type: InstanceType,
    #[serde(rename = "packageLoader")]
    pub package_loader: PackageLoader,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "storePlatform")]
pub enum StorePlatformMetadata {
    Steam {
        #[serde(rename = "storeIdentifier")]
        store_identifier: String,
    },
    SteamDirect {
        #[serde(rename = "storeIdentifier")]
        store_identifier: String,
    },
    #[serde(alias = "Epic Games Store")]
    Epic {
        #[serde(rename = "storeIdentifier")]
        store_identifier: String,
    },
    #[serde(alias = "Xbox Game Pass")]
    Xbox {
        #[serde(rename = "storeIdentifier")]
        store_identifier: String,
    },
    #[serde(alias = "Oculus Store")]
    Oculus,
    #[serde(alias = "Origin / EA Desktop")]
    Origin,
    Other,
}

impl StorePlatformMetadata {
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
