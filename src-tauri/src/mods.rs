use std::ops::Deref;

use rkyv_intern::Intern;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModRef<'a> {
    #[serde(borrow, flatten)]
    pub metadata: ModMetadataRef<'a>,
    #[serde(borrow)]
    pub versions: Vec<ModVersionRef<'a>>,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[rkyv(derive(Debug))]
#[serde(deny_unknown_fields)]
pub struct Mod {
    #[serde(flatten)]
    pub metadata: ModMetadata,
    pub versions: Vec<ModVersion>,
}

impl Deref for Mod {
    type Target = ModMetadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl Deref for ArchivedMod {
    type Target = ArchivedModMetadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl<'a> Deref for ModRef<'a> {
    type Target = ModMetadataRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[rkyv(derive(Debug))]
#[serde(deny_unknown_fields)]
pub struct ModMetadata {
    #[rkyv(with = Intern)]
    pub name: String,
    #[rkyv(with = rkyv::with::Skip)]
    #[serde(skip_serializing)]
    pub full_name: serde::de::IgnoredAny,
    #[rkyv(with = Intern)]
    pub owner: String,
    #[rkyv(with = rkyv::with::Skip)]
    #[serde(skip_serializing)]
    pub package_url: serde::de::IgnoredAny,
    pub donation_link: Option<String>,
    pub date_created: String,
    pub date_updated: String,
    pub rating_score: u32,
    pub is_pinned: bool,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<InternedString>,
    pub uuid4: Uuid,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModMetadataRef<'a> {
    pub name: &'a str,
    #[serde(skip_serializing)]
    pub full_name: serde::de::IgnoredAny,
    pub owner: &'a str,
    #[serde(skip_serializing)]
    pub package_url: serde::de::IgnoredAny,
    pub donation_link: Option<&'a str>,
    pub date_created: &'a str,
    pub date_updated: &'a str,
    pub rating_score: u32,
    pub is_pinned: bool,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<&'a str>,
    pub uuid4: Uuid,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModVersionRef<'a> {
    #[serde(skip_serializing)]
    pub name: serde::de::IgnoredAny,
    #[serde(skip_serializing)]
    pub full_name: serde::de::IgnoredAny,
    pub description: &'a str,
    pub icon: &'a str,
    pub version_number: &'a str,
    #[serde(borrow)]
    pub dependencies: Vec<&'a str>,
    #[serde(skip_serializing)]
    pub download_url: serde::de::IgnoredAny,
    pub downloads: u64,
    pub date_created: &'a str,
    #[serde(borrow)]
    pub website_url: Option<&'a str>,
    pub is_active: bool,
    pub uuid4: Uuid,
    pub file_size: u64,
}

/// See https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/consts.py#L4.
#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize)]
#[rkyv(derive(Debug))]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl serde::Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        const EXPECTED: &str = "a borrowed string of the format MAJOR.MINOR.PATCH";
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Version;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(EXPECTED)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let Some((major, rem)) = v.split_once('.') else {
                    return Err(E::invalid_value(serde::de::Unexpected::Str(v), &EXPECTED));
                };
                let Some((minor, patch)) = rem.split_once('.') else {
                    return Err(E::invalid_value(serde::de::Unexpected::Str(v), &EXPECTED));
                };
                let parse = |s: &str| {
                    s.parse()
                        .map_err(|e| E::invalid_value(serde::de::Unexpected::Str(v), &"an integer"))
                };
                let major = parse(major)?;
                let minor = parse(minor)?;
                let patch = parse(patch)?;
                Ok(Version {
                    major,
                    minor,
                    patch,
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[rkyv(derive(Debug))]
#[serde(deny_unknown_fields)]
pub struct ModVersion {
    #[rkyv(with = rkyv::with::Skip)]
    #[serde(skip_serializing)]
    pub name: serde::de::IgnoredAny,
    #[rkyv(with = rkyv::with::Skip)]
    #[serde(skip_serializing)]
    pub full_name: serde::de::IgnoredAny,
    #[rkyv(with = Intern)]
    pub description: String,
    #[rkyv(with = Intern)]
    pub icon: String,
    #[rkyv(with = Intern)]
    pub version_number: String,
    pub dependencies: Vec<InternedString>,
    #[rkyv(with = rkyv::with::Skip)]
    #[serde(skip_serializing)]
    pub download_url: serde::de::IgnoredAny,
    pub downloads: u64,
    pub date_created: String,
    pub website_url: Option<InternedString>,
    pub is_active: bool,
    pub uuid4: Uuid,
    pub file_size: u64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModAndVersion {
    #[serde(flatten)]
    pub r#mod: ModMetadata,
    pub game: String,
    pub version: ModVersion,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[rkyv(derive(Debug))]
#[serde(transparent)]
pub struct InternedString(#[rkyv(with = Intern)] pub String);

impl Deref for ArchivedInternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}
