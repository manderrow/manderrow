mod spec;
mod timestamp;
mod version;

pub use spec::*;
pub use timestamp::*;
pub use version::*;

use std::ops::Deref;

use rkyv::string::{ArchivedString, StringResolver};
use rkyv::with::NicheInto;
use serde::ser::{SerializeMap, SerializeStruct};
use smol_str::SmolStr;

use crate::util::rkyv::{InternedString, InternedStringNiche, StringIntern};
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
    #[serde(default, skip_serializing)]
    pub full_name: IgnoredAny,
    pub owner: &'a str,
    #[allow(unused)]
    #[serde(default, skip_serializing)]
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
    #[serde(default, skip_serializing)]
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
    #[rkyv(with = NicheInto<InternedStringNiche>)]
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
    #[serde(default, skip_serializing)]
    pub name: IgnoredAny,
    #[allow(unused)]
    #[serde(default, skip_serializing)]
    pub full_name: IgnoredAny,
    pub description: SmolStr,
    #[allow(unused)]
    #[serde(default, skip_serializing)]
    pub icon: IgnoredAny,
    pub version_number: Version,
    #[serde(borrow)]
    pub dependencies: Vec<InternedString<'a>>,
    #[allow(unused)]
    #[serde(default, skip_serializing)]
    pub download_url: IgnoredAny,
    pub downloads: u64,
    pub date_created: Timestamp,
    #[serde(deserialize_with = "empty_string_as_none")]
    pub website_url: Option<SmolStr>,
    pub is_active: bool,
    #[allow(unused)]
    #[serde(default, skip_serializing)]
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
    pub dependencies: Vec<InternedString<'a>>,
    #[allow(unused)]
    #[serde(skip_serializing)]
    pub download_url: IgnoredAny,
    pub downloads: u64,
    pub date_created: Timestamp,
    #[rkyv(with = NicheInto<InternedStringNiche>)]
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
        ser.serialize_field("version_number", &self.version_number.get())?;
        ser.serialize_field("dependencies", &SerializeArchivedVec(&self.dependencies))?;
        ser.serialize_field("downloads", &self.downloads.to_native())?;
        ser.serialize_field("date_created", &Timestamp::from(self.date_created))?;
        ser.serialize_field("website_url", &self.website_url.as_ref())?;
        ser.serialize_field("is_active", &self.is_active)?;
        ser.serialize_field("file_size", &self.file_size.to_native())?;
        ser.end()
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
    use rkyv::primitive::FixedIsize;
    use rkyv::rancor::Strategy;
    use rkyv::string::ArchivedString;
    use rkyv::util::AlignedVec;
    use rkyv::validation::archive::ArchiveValidator;
    use rkyv::validation::shared::SharedValidator;
    use rkyv::validation::Validator;
    use rkyv::with::NicheInto;
    use rkyv_intern::Interner;

    use crate::mods::{
        ArchivedModMetadataRef, ArchivedModVersionRef, ArchivedVersion, InlineString,
        InternedString, ModMetadataRef, ModRef, ModVersionRef,
    };
    use crate::util::rkyv::{ArchivedInternedString, InternedStringNiche};

    use super::Version;

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
            panic!("{e}\n{buf:?}\n{buf:x?}")
        }

        buf
    }

    #[test]
    fn test_sizes() {
        assert_eq!(size_of::<ArchivedVersion>(), size_of::<u32>());
        assert_eq!(size_of::<ArchivedString>(), size_of::<usize>());
        assert_eq!(size_of::<ArchivedInternedString>(), size_of::<FixedIsize>());
        assert_eq!(size_of::<ArchivedModMetadataRef>(), 48);
        assert_eq!(size_of::<ArchivedModVersionRef>(), 48);
    }

    #[test]
    fn test_inline_string_encoding() {
        let buf = serialize::<_, String>(&InlineString("BepInEx"));
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
    }

    #[test]
    fn test_interned_string_encoding() {
        #[derive(rkyv::Archive, rkyv::Serialize)]
        struct OptionInternedString<'a>(
            #[rkyv(with = NicheInto<InternedStringNiche>)] Option<InternedString<'a>>,
        );

        impl<'a> OptionInternedString<'a> {
            pub fn some(t: InternedString<'a>) -> Self {
                Self(Some(t))
            }

            pub fn none() -> Self {
                Self(None)
            }
        }

        let buf = serialize::<_, String>(&InternedString("Abcd"));
        assert_eq!(
            buf.as_slice(),
            b"Abcd",
            "Tiny string should be serialized inline"
        );

        let buf = serialize::<_, String>(&InternedString("Ab"));
        assert_eq!(
            buf.as_slice(),
            b"Ab\xff\x00",
            "Tiny string should be serialized inline"
        );

        let buf = serialize::<_, String>(&InternedString("BepInEx"));
        assert_eq!(
            buf.as_slice(),
            b"\x07\0\0\0BepInEx\0\xb7\xff\xff\xff",
            "Short string should be serialized out-of-line"
        );

        let buf = serialize::<_, String>(&InternedString("BepInExPack"));
        assert_eq!(
            buf.as_slice(),
            b"\x0b\0\0\0BepInExPack\0\xb3\xff\xff\xff",
            "Long string should be serialized out-of-line"
        );

        let buf = serialize::<_, String>(&OptionInternedString::none());
        assert_eq!(
            buf.as_slice(),
            b"\xc0\0\0\0",
            "None should be serialized correctly"
        );

        let buf = serialize::<_, String>(&OptionInternedString::some(InternedString("Abcd")));
        assert_eq!(
            buf.as_slice(),
            b"Abcd",
            "Some(...) with inline string be serialized correctly, zero overhead"
        );

        let buf =
            serialize::<_, String>(&OptionInternedString::some(InternedString("BepInExPack")));
        assert_eq!(
            buf.as_slice(),
            b"\x0b\0\0\0BepInExPack\0\xb3\xff\xff\xff",
            "Some(...) with out-of-line string be serialized correctly, zero overhead"
        );

        let buf = serialize::<_, String>(&InternedString(""));
        assert_eq!(
            buf.as_slice(),
            b"\xff\0\0\0",
            "Empty string should be serialized correctly, zero overhead"
        );

        let buf = serialize::<_, String>(&InternedString("https://github.com/ebkr/r2modmanPlus"));
        assert_eq!(
            buf.as_slice(),
            b"\x24\0\0\0https://github.com/ebkr/r2modmanPlus\x9b\xff\xff\xff",
            "This url should be serialized correctly"
        );

        let buf = serialize::<_, String>(&[
            InternedString("https://github.com/ebkr/r2modmanPlus"),
            InternedString("https://github.com/ebkr/r2modmanPlus"),
        ]);
        assert_eq!(
            buf.as_slice(),
            b"\x24\0\0\0https://github.com/ebkr/r2modmanPlus\x9b\xff\xff\xff\x97\xff\xff\xff",
            "Repeated interned strings should be serialized correctly"
        );
    }

    #[test]
    fn test_encoding() {
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
