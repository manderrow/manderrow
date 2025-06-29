use std::borrow::Cow;

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
    #[serde(rename = "exeNames", borrow)]
    pub exe_names: Vec<Cow<'a, str>>,
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
        #[serde(rename = "storePageIdentifier", borrow, default)]
        store_page_identifier: Option<Cow<'a, str>>,
    },
    SteamDirect {
        #[serde(rename = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
        #[serde(rename = "storePageIdentifier", borrow, default)]
        store_page_identifier: Option<Cow<'a, str>>,
    },
    Epic {
        #[serde(rename = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    Xbox {
        #[serde(rename = "storeIdentifier", borrow)]
        store_identifier: Cow<'a, str>,
    },
    Oculus,
    Origin,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub struct SteamMetadata<'a> {
    pub id: &'a str,
    pub page_id: Option<&'a str>,
}

impl<'a> StorePlatformMetadata<'a> {
    pub fn steam_or_direct(&self) -> Option<SteamMetadata> {
        match self {
            StorePlatformMetadata::Steam {
                store_identifier,
                store_page_identifier,
            }
            | StorePlatformMetadata::SteamDirect {
                store_identifier,
                store_page_identifier,
            } => Some(SteamMetadata {
                id: &store_identifier,
                page_id: store_page_identifier.as_deref(),
            }),
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
