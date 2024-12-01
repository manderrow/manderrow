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
    #[serde(alias = "displayName")]
    pub name: String,
    /// URL of the Thunderstore mod index for the game.
    #[serde(alias = "thunderstoreUrl")]
    pub thunderstore_url: String,
    #[serde(alias = "steamFolderName")]
    pub steam_folder_name: String,
    #[serde(alias = "settingsIdentifier")]
    pub settings_identifier: String,
    #[serde(alias = "exeName")]
    pub exe_names: Vec<String>,
    #[serde(alias = "dataFolderName")]
    pub data_folder_name: String,
    #[serde(alias = "exclusionsUrl")]
    pub exclusions_url: String,
    #[serde(alias = "storePlatformMetadata")]
    pub store_platform_metadata: Vec<StorePlatformMetadata>,
    #[serde(alias = "gameImage")]
    pub game_image: String,
    #[serde(alias = "displayMode")]
    pub display_mode: usize,
    #[serde(alias = "instanceType")]
    pub instance_type: String,
    #[serde(alias = "packageLoader")]
    pub package_loader: PackageLoader,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "_storePlatform")]
pub enum StorePlatformMetadata {
    Steam {
        #[serde(alias = "_storeIdentifier")]
        store_identifier: String,
    },
    SteamDirect {
        #[serde(alias = "_storeIdentifier")]
        store_identifier: String,
    },
    #[serde(alias = "Epic Games Store")]
    Epic {
        #[serde(alias = "_storeIdentifier")]
        store_identifier: String,
    },
    #[serde(alias = "Xbox Game Pass")]
    Xbox {
        #[serde(alias = "_storeIdentifier")]
        store_identifier: String,
    },
    #[serde(alias = "Oculus Store")]
    Oculus,
    #[serde(alias = "Origin / EA Desktop")]
    Origin,
    Other,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
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
