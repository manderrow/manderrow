use std::ops::Deref;

use rkyv::with::InlineAsBox;
use rkyv_intern::{DerefIntern, Intern};
use smol_str::SmolStr;
use uuid::Uuid;

use crate::util::serde::IgnoredAny;

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
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

impl<'a> Deref for ArchivedModRef<'a> {
    type Target = ArchivedModMetadataRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[rkyv(derive(Debug))]
#[serde(deny_unknown_fields)]
pub struct ModMetadata {
    #[rkyv(with = Intern)]
    pub name: SmolStr,
    #[serde(skip_serializing)]
    pub full_name: IgnoredAny,
    #[rkyv(with = Intern)]
    pub owner: SmolStr,
    #[serde(skip_serializing)]
    pub package_url: IgnoredAny,
    pub donation_link: Option<String>,
    pub date_created: SmolStr,
    pub date_updated: SmolStr,
    pub rating_score: u32,
    pub is_pinned: bool,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<InternedString>,
    pub uuid4: Uuid,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModMetadataRef<'a> {
    #[rkyv(with = DerefIntern)]
    pub name: &'a str,
    #[serde(skip_serializing)]
    pub full_name: IgnoredAny,
    #[rkyv(with = DerefIntern)]
    pub owner: &'a str,
    #[serde(skip_serializing)]
    pub package_url: IgnoredAny,
    pub donation_link: Option<InlineStringRef<'a>>,
    #[rkyv(with = InlineAsBox)]
    pub date_created: &'a str,
    #[rkyv(with = InlineAsBox)]
    pub date_updated: &'a str,
    pub rating_score: u32,
    pub is_pinned: bool,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<InternedStringRef<'a>>,
    pub uuid4: Uuid,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[rkyv(derive(Debug))]
#[serde(deny_unknown_fields)]
pub struct ModVersion {
    #[serde(skip_serializing)]
    pub name: IgnoredAny,
    #[serde(skip_serializing)]
    pub full_name: IgnoredAny,
    #[rkyv(with = Intern)]
    pub description: SmolStr,
    #[serde(skip_serializing)]
    pub icon: IgnoredAny,
    pub version_number: Version,
    pub dependencies: Vec<InternedString>,
    #[serde(skip_serializing)]
    pub download_url: IgnoredAny,
    pub downloads: u64,
    pub date_created: SmolStr,
    pub website_url: Option<InternedString>,
    pub is_active: bool,
    pub uuid4: Uuid,
    pub file_size: u64,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModVersionRef<'a> {
    #[serde(skip_serializing)]
    pub name: IgnoredAny,
    #[serde(skip_serializing)]
    pub full_name: IgnoredAny,
    #[rkyv(with = DerefIntern)]
    pub description: &'a str,
    #[serde(skip_serializing)]
    pub icon: IgnoredAny,
    pub version_number: Version,
    #[serde(borrow)]
    pub dependencies: Vec<InternedStringRef<'a>>,
    #[serde(skip_serializing)]
    pub download_url: IgnoredAny,
    pub downloads: u64,
    #[rkyv(with = InlineAsBox)]
    pub date_created: &'a str,
    #[serde(borrow)]
    pub website_url: Option<InternedStringRef<'a>>,
    pub is_active: bool,
    pub uuid4: Uuid,
    pub file_size: u64,
}

/// See https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/consts.py#L4.
/// and https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/models/package_version.py#L101-L103
#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize)]
#[rkyv(derive(Clone, Copy))]
pub struct Version(u64);

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Option<Self> {
        let minor_shift = u64::BITS - patch.leading_zeros();
        let major_shift = minor_shift + u64::BITS - minor.leading_zeros();
        let len = major_shift + u64::BITS - major.leading_zeros();
        const MAX_LEN: u32 = 16;
        const MAX_DIGITS: u32 = MAX_LEN - 2;
        // can't be const yet
        // const MAX_BITS: u32 = ((((MAX_DIGITS as f64) / 3.0) / 2.0f64.log10()).ceil() as u32) * 3;
        const MAX_BITS: u32 = 48;
        if len > MAX_BITS {
            // upper bound should be 47 bits
            return None;
        }
        Some(Self(
            (major << major_shift)
                | (minor << minor_shift)
                | patch
                | (u64::from(major_shift) << (MAX_BITS + u8::BITS))
                | (u64::from(minor_shift) << MAX_BITS),
        ))
    }

    pub fn major(self) -> u64 {
        let major_shift = (self.0 >> 56) as u32;
        (self.0 & 0xFFFF_FFFF_FFFF).unbounded_shr(major_shift)
    }

    pub fn minor(self) -> u64 {
        let major_shift = (self.0 >> 56) as u32;
        let minor_shift = ((self.0 >> 48) & 0xFF) as u32;
        self.0
            .unbounded_shl(64 - major_shift)
            .unbounded_shr(64 - major_shift + minor_shift)
    }

    pub fn patch(self) -> u64 {
        let minor_shift = ((self.0 >> 48) & 0xFF) as u32;
        self.0
            .unbounded_shl(64 - minor_shift)
            .unbounded_shr(64 - minor_shift)
    }
}

impl From<ArchivedVersion> for Version {
    fn from(value: ArchivedVersion) -> Self {
        Self(value.0.into())
    }
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
        const EXPECTED: &str =
            "a borrowed string of the format MAJOR.MINOR.PATCH, up to 16 characters (inclusive)";
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
                    s.parse::<u64>().map_err(|e| {
                        E::custom(format_args!(
                            "invalid value: {} ({e})",
                            serde::de::Unexpected::Str(v)
                        ))
                    })
                };
                let major = parse(major)?;
                let minor = parse(minor)?;
                let patch = parse(patch)?;
                Version::new(major, minor, patch)
                    .ok_or_else(|| E::invalid_value(serde::de::Unexpected::Str(v), &EXPECTED))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for ArchivedVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&Version::from(*self), f)
    }
}

impl std::fmt::Debug for ArchivedVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModAndVersion {
    #[serde(flatten)]
    pub r#mod: ModMetadata,
    pub game: SmolStr,
    pub version: ModVersion,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[rkyv(derive(Debug))]
#[serde(transparent)]
pub struct InternedString(#[rkyv(with = Intern)] pub SmolStr);

impl Deref for ArchivedInternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct InternedStringRef<'a>(#[rkyv(with = DerefIntern)] pub &'a str);

impl<'a> Deref for ArchivedInternedStringRef<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct InlineStringRef<'a>(#[rkyv(with = InlineAsBox)] pub &'a str);

impl<'a> Deref for ArchivedInlineStringRef<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn test_version() {
        #[track_caller]
        fn case(major: u64, minor: u64, patch: u64) {
            let version = Version::new(major, minor, patch).unwrap();
            assert_eq!(version.major(), major, "major version mismatch");
            assert_eq!(version.minor(), minor, "minor version mismatch");
            assert_eq!(version.patch(), patch, "patch version mismatch");
        }
        case(31251241231, 0, 0);
        case(69, 201, 131125);
    }
}
