mod spec;
mod timestamp;

pub use spec::*;
pub use timestamp::*;

use std::num::ParseIntError;
use std::ops::Deref;

use rkyv::string::{ArchivedString, StringResolver};
use rkyv::with::NicheInto;
use serde::ser::{SerializeMap, SerializeStruct};
use smol_str::SmolStr;

use crate::util::rkyv::{InternedString, StringIntern, FE};
use crate::util::serde::{empty_string_as_none, IgnoredAny, SerializeArchivedVec};

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModRef<'a> {
    #[serde(flatten)]
    pub metadata: ModMetadataRef<'a>,
    #[serde(borrow)]
    pub versions: Vec<ModVersionRef<'a>>,
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

impl<'a> serde::Serialize for ArchivedModRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_map(Some(11))?;
        self.metadata
            .serialize(serde::__private::ser::FlatMapSerializer(&mut ser))?;
        ser.serialize_entry("versions", &SerializeArchivedVec(&self.versions))?;
        ser.end()
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModMetadata<'a> {
    pub name: &'a str,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub full_name: IgnoredAny,
    pub owner: &'a str,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub package_url: IgnoredAny,
    #[serde(deserialize_with = "empty_string_as_none")]
    pub donation_link: Option<SmolStr>,
    pub date_created: Timestamp,
    pub date_updated: Timestamp,
    pub rating_score: u32,
    pub is_pinned: bool,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<SmolStr>,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub uuid4: IgnoredAny,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModMetadataRef<'a> {
    #[rkyv(with = StringIntern)]
    pub name: &'a str,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub full_name: IgnoredAny,
    #[rkyv(with = StringIntern)]
    pub owner: &'a str,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub package_url: IgnoredAny,
    #[rkyv(with = NicheInto<FE>)]
    #[serde(deserialize_with = "empty_string_as_none")]
    pub donation_link: Option<InternedString<'a>>,
    pub date_created: Timestamp,
    pub date_updated: Timestamp,
    pub rating_score: u32,
    pub is_pinned: bool,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    pub categories: Vec<InternedString<'a>>,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub uuid4: IgnoredAny,
}

impl<'a> serde::Serialize for ArchivedModMetadataRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_struct("ModMetadataRef", 10)?;
        ser.serialize_field("name", &self.name)?;
        ser.serialize_field("owner", &self.owner)?;
        ser.serialize_field("donation_link", &self.donation_link.as_deref())?;
        ser.serialize_field("date_created", &Timestamp::from(self.date_created))?;
        ser.serialize_field("date_updated", &Timestamp::from(self.date_updated))?;
        ser.serialize_field("rating_score", &self.rating_score.to_native())?;
        ser.serialize_field("is_pinned", &self.is_pinned)?;
        ser.serialize_field("is_deprecated", &self.is_deprecated)?;
        ser.serialize_field("has_nsfw_content", &self.has_nsfw_content)?;
        ser.serialize_field("categories", &SerializeArchivedVec(&self.categories))?;
        ser.end()
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModVersion<'a> {
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub name: IgnoredAny,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub full_name: IgnoredAny,
    pub description: SmolStr,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub icon: IgnoredAny,
    pub version_number: Version,
    #[serde(borrow)]
    pub dependencies: Vec<ModSpec<'a>>,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub download_url: IgnoredAny,
    pub downloads: u64,
    pub date_created: Timestamp,
    #[serde(deserialize_with = "empty_string_as_none")]
    pub website_url: Option<SmolStr>,
    pub is_active: bool,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub uuid4: IgnoredAny,
    pub file_size: u64,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModVersionRef<'a> {
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub name: IgnoredAny,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub full_name: IgnoredAny,
    #[rkyv(with = StringIntern)]
    pub description: &'a str,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub icon: IgnoredAny,
    pub version_number: Version,
    #[serde(borrow)]
    pub dependencies: Vec<ModSpec<'a>>,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub download_url: IgnoredAny,
    pub downloads: u64,
    pub date_created: Timestamp,
    #[rkyv(with = NicheInto<FE>)]
    #[serde(deserialize_with = "empty_string_as_none")]
    pub website_url: Option<InternedString<'a>>,
    pub is_active: bool,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub uuid4: IgnoredAny,
    pub file_size: u64,
}

impl<'a> serde::Serialize for ArchivedModVersionRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_struct("ModVersionRef", 8)?;
        ser.serialize_field("description", &self.description)?;
        ser.serialize_field("version_number", &Version::from(self.version_number))?;
        ser.serialize_field("dependencies", &SerializeArchivedVec(&self.dependencies))?;
        ser.serialize_field("downloads", &self.downloads.to_native())?;
        ser.serialize_field("date_created", &Timestamp::from(self.date_created))?;
        ser.serialize_field("website_url", &self.website_url.as_ref())?;
        ser.serialize_field("is_active", &self.is_active)?;
        ser.serialize_field("file_size", &self.file_size.to_native())?;
        ser.end()
    }
}

/// See https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/consts.py#L4.
/// and https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/models/package_version.py#L101-L103
#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize)]
#[rkyv(derive(Clone, Copy))]
pub struct Version(u64);

impl Version {
    const MAX_LEN: u32 = 16;

    pub fn new(major: u64, minor: u64, patch: u64) -> Option<Self> {
        let minor_shift = u64::BITS - patch.leading_zeros();
        let major_shift = minor_shift + u64::BITS - minor.leading_zeros();
        let len = major_shift + u64::BITS - major.leading_zeros();
        const MAX_DIGITS: u32 = Version::MAX_LEN - 2;
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

#[derive(Debug, thiserror::Error)]
pub enum VersionParseError<'a> {
    #[error(
        "too long: {value:?}, expected at most {} characters",
        Version::MAX_LEN
    )]
    TooLong { value: &'a str },
    #[error("missing dot: {value:?}, expected 2, found {found}")]
    MissingDot { value: &'a str, found: usize },
    #[error("invalid integer: {value:?}, specifically {slice:?}, {error}")]
    InvalidInteger {
        value: &'a str,
        slice: &'a str,
        #[source]
        error: ParseIntError,
    },
}

impl Version {
    pub fn from_str(value: &str) -> Result<Self, VersionParseError<'_>> {
        if value.len() > Self::MAX_LEN as usize {
            return Err(VersionParseError::TooLong { value });
        }
        let Some((major, rem)) = value.split_once('.') else {
            return Err(VersionParseError::MissingDot { value, found: 0 });
        };
        let Some((minor, patch)) = rem.split_once('.') else {
            return Err(VersionParseError::MissingDot { value, found: 1 });
        };
        fn parse<'a>(value: &'a str, slice: &'a str) -> Result<u64, VersionParseError<'a>> {
            slice
                .parse::<u64>()
                .map_err(|error| VersionParseError::InvalidInteger {
                    value,
                    slice,
                    error,
                })
        }
        let major = parse(value, major)?;
        let minor = parse(value, minor)?;
        let patch = parse(value, patch)?;
        Ok(Version::new(major, minor, patch).unwrap())
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
                Version::from_str(v).map_err(E::custom)
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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModAndVersion<'a> {
    #[serde(flatten)]
    pub r#mod: ModMetadata<'a>,
    pub version: ModVersion<'a>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct InlineString<'a>(pub &'a str);

impl<'a> From<&'a str> for InlineString<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl Deref for InlineString<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl rkyv::Archive for InlineString<'_> {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        ArchivedString::resolve_from_str(self.0, resolver, out);
    }
}

impl<S> rkyv::Serialize<S> for InlineString<'_>
where
    S: rkyv::rancor::Fallible + ?Sized,
    S::Error: rkyv::rancor::Source,
    str: rkyv::SerializeUnsized<S>,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(self.0, serializer)
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use rkyv::niche::niching::Niching;
    use rkyv::primitive::{FixedIsize, FixedUsize};
    use rkyv::rancor::Strategy;
    use rkyv::string::ArchivedString;
    use rkyv::util::AlignedVec;
    use rkyv::validation::archive::ArchiveValidator;
    use rkyv::validation::shared::SharedValidator;
    use rkyv::validation::Validator;
    use rkyv::with::NicheInto;
    use rkyv_intern::Interner;
    use smol_str::SmolStr;
    use uuid::Uuid;

    use crate::mods::{
        ArchivedModMetadataRef, ArchivedModVersionRef, ArchivedVersion, InlineString,
        InternedString, ModMetadata, ModMetadataRef, ModRef, ModVersion, ModVersionRef,
    };
    use crate::util::rkyv::{ArchivedInternedString, FE};

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

    type Serializer<'a, I> = rkyv::rancor::Strategy<
        rkyv_intern::InterningAdapter<
            rkyv::ser::Serializer<
                rkyv::util::AlignedVec<16>,
                rkyv::ser::allocator::ArenaHandle<'a>,
                rkyv::ser::sharing::Share,
            >,
            Interner<I>,
        >,
        rkyv::rancor::Error,
    >;

    #[track_caller]
    fn serialize<T, I>(value: &T) -> AlignedVec
    where
        T: for<'a> rkyv::Serialize<Serializer<'a, I>>,
        T::Archived: for<'a> rkyv::bytecheck::CheckBytes<
            Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, rkyv::rancor::Error>,
        >,
    {
        let buf = rkyv::util::with_arena(|arena| {
            let mut serializer = rkyv_intern::InterningAdapter::new(
                rkyv::ser::Serializer::new(
                    rkyv::util::AlignedVec::<16>::new(),
                    arena.acquire(),
                    rkyv::ser::sharing::Share::new(),
                ),
                Interner::<I>::default(),
            );
            rkyv::api::serialize_using::<_, rkyv::rancor::Error>(value, &mut serializer)?;
            let (serializer, _interner) = serializer.into_components();
            Ok::<_, rkyv::rancor::Error>(serializer.into_writer())
        })
        .unwrap();

        if let Err(e) = rkyv::access::<T::Archived, rkyv::rancor::Error>(&buf) {
            panic!("{e}\n{buf:?}")
        }

        buf
    }

    #[test]
    fn test_sizes() {
        assert_eq!(size_of::<ArchivedVersion>(), size_of::<u64>());
        assert_eq!(size_of::<ArchivedString>(), size_of::<usize>());
        assert_eq!(
            size_of::<ArchivedInternedString>(),
            size_of::<FixedIsize>() + size_of::<FixedUsize>()
        );
        assert_eq!(size_of::<ArchivedModMetadataRef>(), 56);
        assert_eq!(size_of::<ArchivedModVersionRef>(), 64);

        let buf = serialize::<_, String>(&InlineString("BepInEx"));
        assert_eq!(
            buf.as_slice(),
            b"BepInEx\xff",
            "Short string should be serialized inline"
        );

        let buf = serialize::<_, String>(&InternedString("BepInEx"));
        assert_eq!(
            buf.as_slice(),
            b"BepInEx\xff",
            "Short string should be serialized inline"
        );

        let buf = serialize::<_, String>(&InlineString("BepInExPack"));
        assert_eq!(
            buf.as_slice(),
            b"BepInExPack\0\x8b\0\0\0\xf4\xff\xff\xff",
            "Long string should be serialized out-of-line"
        );

        let buf = serialize::<_, String>(&InternedString("BepInExPack"));
        assert_eq!(
            buf.as_slice(),
            b"BepInExPack\0\x8b\0\0\0\xf4\xff\xff\xff",
            "Long string should be serialized out-of-line"
        );

        #[derive(rkyv::Archive, rkyv::Serialize)]
        struct NichedOption<T: rkyv::Archive>(#[rkyv(with = NicheInto<FE>)] Option<T>)
        where
            FE: Niching<T::Archived>;

        impl<T: rkyv::Archive> NichedOption<T>
        where
            FE: Niching<T::Archived>,
        {
            pub fn some(t: T) -> Self {
                Self(Some(t))
            }

            pub fn none() -> Self {
                Self(None)
            }
        }

        let buf = serialize::<_, String>(&NichedOption::some(InlineString("BepInExPack")));
        assert_eq!(
            buf.as_slice(),
            b"BepInExPack\0\x8b\0\0\0\xf4\xff\xff\xff",
            "Option<InlineString> should be zero overhead"
        );

        let buf = serialize::<_, String>(&NichedOption::some(InternedString("BepInExPack")));
        assert_eq!(
            buf.as_slice(),
            b"BepInExPack\0\x8b\0\0\0\xf4\xff\xff\xff",
            "Option<InternedString> should be zero overhead"
        );

        let buf = serialize::<_, String>(&NichedOption::some(InternedString("")));
        assert_eq!(
            buf.as_slice(),
            b"\xff\xff\xff\xff\xff\xff\xff\xff",
            "Empty Option<InternedString> should encode correctly"
        );

        let buf = serialize::<_, String>(&InternedString("https://github.com/ebkr/r2modmanPlus"));
        assert_eq!(
            buf.as_slice(),
            b"https://github.com/ebkr/r2modmanPlus\xa4\0\0\0\xdc\xff\xff\xff",
            "This url should encode correctly"
        );

        let buf = serialize::<_, String>(&[
            InternedString("https://github.com/ebkr/r2modmanPlus"),
            InternedString("https://github.com/ebkr/r2modmanPlus"),
        ]);
        assert_eq!(
            buf.as_slice(),
            b"https://github.com/ebkr/r2modmanPlus\xa4\0\0\0\xdc\xff\xff\xff\xa4\0\0\0\xd4\xff\xff\xff",
            "Interned strings repeated should encode correctly"
        );

        let buf = serialize::<_, String>(&[ModRef {
            metadata: ModMetadataRef {
                name: "BepInExPack",
                full_name: Default::default(),
                owner: "BepInEx",
                package_url: Default::default(),
                donation_link: None,
                date_created: "2023-01-17T16:24:38.370139Z".parse().unwrap(),
                date_updated: "2023-01-17T16:24:39.204947Z".parse().unwrap(),
                rating_score: 413,
                is_pinned: true,
                is_deprecated: false,
                has_nsfw_content: false,
                categories: vec!["Libraries".into()],
                uuid4: Default::default(),
            },
            versions: vec![ModVersionRef {
                name: Default::default(),
                full_name: Default::default(),
                description: "BepInEx pack for Mono Unity games. Preconfigured and ready to use.",
                icon: Default::default(),
                version_number: Version::from_str("5.4.2100").unwrap(),
                dependencies: vec![],
                download_url: Default::default(),
                downloads: 15784758,
                date_created: "2023-01-17T16:24:38.784605Z".parse().unwrap(),
                website_url: Some("https://github.com/BepInEx/BepInEx".into()),
                is_active: true,
                uuid4: Default::default(),
                file_size: 0,
            }],
        }]);
        assert_eq!(buf.len(), 264);
    }
}
