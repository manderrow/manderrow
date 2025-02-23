use std::{fmt::{self, Formatter}, num::ParseIntError};

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

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
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

impl fmt::Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for ArchivedVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&Version::from(*self), f)
    }
}

impl fmt::Debug for ArchivedVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}