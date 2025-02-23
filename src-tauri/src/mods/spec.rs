use std::{fmt, ops::Deref};

use rkyv_intern::Intern;

use crate::util::rkyv::InternedString;

use super::{Version, VersionParseError};

#[derive(Debug, thiserror::Error)]
pub enum ModIdParseError<'a> {
    #[error("missing delimiter: {value:?}")]
    MissingDelimiter { value: &'a str },
}

#[derive(Debug, thiserror::Error)]
pub enum ModSpecParseError<'a> {
    #[error("missing delimiter: {value:?}")]
    MissingDelimiter { value: &'a str },
    #[error("invalid id: {value:?}, {error}")]
    InvalidId {
        value: &'a str,
        error: ModIdParseError<'a>,
    },
    #[error("invalid version: {value:?}, {error}")]
    InvalidVersion {
        value: &'a str,
        error: VersionParseError<'a>,
    },
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize)]
pub struct ModId<'a> {
    owner: InternedString<'a>,
    name: InternedString<'a>,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize)]
pub struct ModSpec<'a> {
    id: InternedString<'a>,
    pub version: Version,
}

impl<'a> ModId<'a> {
    pub fn from_str(value: &'a str) -> Result<Self, ModIdParseError<'a>> {
        let Some((owner, name)) = value.rsplit_once('-') else {
            return Err(ModIdParseError::MissingDelimiter { value });
        };
        Ok(Self {
            owner: owner.into(),
            name: name.into(),
        })
    }
}

impl<'a> ModSpec<'a> {
    pub fn from_str(value: &'a str) -> Result<Self, ModSpecParseError<'a>> {
        let Some((rem, version)) = value.rsplit_once('-') else {
            return Err(ModSpecParseError::MissingDelimiter { value });
        };
        let version = Version::from_str(version)
            .map_err(|error| ModSpecParseError::InvalidVersion { value, error })?;
        ModId::from_str(rem).map_err(|error| ModSpecParseError::InvalidId { value, error })?;
        Ok(Self {
            id: rem.into(),
            version,
        })
    }
}

impl<'a> From<&'a ArchivedModId<'_>> for ModId<'a> {
    fn from(value: &'a ArchivedModId<'_>) -> Self {
        Self {
            owner: (&value.owner).into(),
            name: (&value.name).into(),
        }
    }
}

impl<'a> From<&'a ArchivedModSpec<'_>> for ModSpec<'a> {
    fn from(value: &'a ArchivedModSpec<'_>) -> Self {
        Self {
            id: (&value.id).into(),
            version: value.version.into(),
        }
    }
}

impl<'a> ModSpec<'a> {
    fn id(&self) -> ModId<'_> {
        ModId::from_str(&self.id).unwrap()
    }
}

impl<'a> ArchivedModSpec<'a> {
    fn id(&self) -> ModId<'_> {
        ModId::from_str(&self.id).unwrap()
    }
}

impl fmt::Display for ModId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", &*self.owner, &*self.name)
    }
}

impl fmt::Display for ModSpec<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", &*self.id, self.version)
    }
}

impl serde::Serialize for ModSpec<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de: 'a, 'a> serde::Deserialize<'de> for ModSpec<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(Visitor)
    }
}

impl<'a> serde::Serialize for ArchivedModSpec<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ModSpec::from(self).serialize(serializer)
    }
}

struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = ModSpec<'de>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a borrowed string of the format OWNER-NAME-VERSION")
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        ModSpec::from_str(v).map_err(E::custom)
    }
}
