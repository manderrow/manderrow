use std::{
    fmt::{self, Formatter},
    mem::ManuallyDrop,
    num::ParseIntError,
};

#[cfg(feature = "statistics")]
use std::sync::atomic::AtomicU32;

use rkyv::{
    bytecheck::CheckBytes,
    munge::munge,
    primitive::{ArchivedU32, ArchivedU64, FixedUsize},
    rancor::{fail, Fallible, Source},
    ser::Writer,
    Portable, RelPtr, SerializeUnsized,
};

/// Returns the minimum number of bits required to store the value.
fn bit_len(value: u64) -> u32 {
    u64::BITS - value.leading_zeros()
}

#[derive(Debug, thiserror::Error)]
#[error("Too many bits in version")]
pub struct TooManyBitsError;

struct Components {
    digits: u64,
    minor_exp: u32,
    major_exp: u32,
}

impl Components {
    #[inline]
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        let minor_exp = patch.checked_ilog10().unwrap_or(0) + 1;
        let major_exp = minor.checked_ilog10().unwrap_or(0) + 1;

        let minor_mul = 10u64.pow(minor_exp);
        let major_mul = 10u64.pow(major_exp);

        Self {
            digits: patch + minor * minor_mul + major * minor_mul * major_mul,
            minor_exp,
            major_exp,
        }
    }
}

#[derive(Clone, Copy)]
struct Packer {
    digit_bits: u32,
    index_bits: u32,
    digits_mask: u64,
    index_mask: u8,
}

impl Packer {
    #[inline]
    pub fn pack(&self, components: Components) -> Result<u64, TooManyBitsError> {
        let Components {
            digits,
            minor_exp,
            major_exp,
        } = components;

        if bit_len(digits) > self.digit_bits {
            return Err(TooManyBitsError);
        }

        debug_assert!(bit_len(minor_exp.into()) <= self.index_bits);
        debug_assert!(bit_len(major_exp.into()) <= self.index_bits);

        // println!("{major_exp:b} {minor_exp:b} {digits:b}");

        Ok(digits << 2
            | (u64::from(major_exp) << (2 + self.digit_bits + self.index_bits))
            | (u64::from(minor_exp) << (2 + self.digit_bits)))
    }

    #[inline]
    pub fn unpack(self, value: u64) -> (u64, u64, u64) {
        // discard marker
        let value = value >> 2;

        let minor_exp = (((value >> self.digit_bits) as u8) & self.index_mask) as u32;
        let major_exp =
            (((value >> (self.digit_bits + self.index_bits)) as u8) & self.index_mask) as u32;
        let digits = value & self.digits_mask;

        let minor_mul = 10u64.pow(minor_exp);
        let major_mul = 10u64.pow(major_exp);

        // println!("{major_exp:b} {minor_exp:b} {digits:b}");

        (
            digits / minor_mul / major_mul,
            (digits / minor_mul) % major_mul,
            digits % minor_mul,
        )
    }
}

const INLINE_MARKER: u8 = 0b01;

const INLINE_PACKER: Packer = Packer {
    digit_bits: 24,
    index_bits: 3,
    digits_mask: 0xff_ff_ff,
    index_mask: 0b111,
};

const OUT_OF_LINE_PACKER: Packer = Packer {
    digit_bits: 47,
    index_bits: 4,
    digits_mask: 0x7f_ff_ff_ff_ff_ff,
    index_mask: 0b1111,
};

/// See https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/consts.py#L4.
/// and https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/models/package_version.py#L101-L103
///
/// The [`PartialEq`], [`Eq`], and [`Hash`] trait impls rely on there being a single canonical representation that is always used for a given version.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version(u64);

impl Version {
    const MAX_LEN: u32 = 16;

    pub fn new(major: u64, minor: u64, patch: u64) -> Result<Self, TooManyBitsError> {
        let components = Components::new(major, minor, patch);
        if bit_len(components.digits) <= INLINE_PACKER.digit_bits {
            Ok(Self(
                INLINE_PACKER.pack(components).unwrap() | u64::from(INLINE_MARKER),
            ))
        } else {
            Ok(Self(OUT_OF_LINE_PACKER.pack(components)?))
        }
    }

    fn is_inline(self) -> bool {
        self.0 & u64::from(INLINE_MARKER) != 0
    }

    pub fn components(self) -> (u64, u64, u64) {
        if self.is_inline() {
            INLINE_PACKER.unpack(self.0)
        } else {
            OUT_OF_LINE_PACKER.unpack(self.0)
        }
    }

    pub fn major(self) -> u64 {
        let (major, _, _) = self.components();
        major
    }

    pub fn minor(self) -> u64 {
        let (_, minor, _) = self.components();
        minor
    }

    pub fn patch(self) -> u64 {
        let (_, _, patch) = self.components();
        patch
    }
}

pub struct VersionResolver {
    pos: FixedUsize,
}

impl rkyv::Archive for Version {
    type Archived = ArchivedVersion;

    type Resolver = VersionResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        if self.is_inline() {
            unsafe {
                out.write_unchecked(ArchivedVersion {
                    repr: ArchivedVersionRepr {
                        inline: ArchivedU32::from_native(self.0 as u32),
                    },
                })
            };
        } else {
            munge!(let ArchivedVersion { repr: ArchivedVersionRepr { out_of_line } } = out);
            let out_of_line = unsafe { out_of_line.cast_unchecked::<RelPtr<ArchivedU64>>() };
            RelPtr::emplace(resolver.pos as usize, out_of_line);
        }
    }
}

#[cfg(feature = "statistics")]
static INLINE_COUNT: AtomicU32 = AtomicU32::new(0);
#[cfg(feature = "statistics")]
static OUT_OF_LINE_COUNT: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "statistics")]
pub fn reset_version_repr_stats() {
    INLINE_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
    OUT_OF_LINE_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(feature = "statistics")]
pub fn get_version_repr_stats() -> (u32, u32) {
    (
        INLINE_COUNT.load(std::sync::atomic::Ordering::Relaxed),
        OUT_OF_LINE_COUNT.load(std::sync::atomic::Ordering::Relaxed),
    )
}

impl<S: Fallible + Writer + ?Sized> rkyv::Serialize<S> for Version {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        if self.is_inline() {
            #[cfg(feature = "statistics")]
            INLINE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(VersionResolver { pos: 0 })
        } else {
            #[cfg(feature = "statistics")]
            OUT_OF_LINE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(VersionResolver {
                pos: self.0.serialize_unsized(serializer)?.try_into().unwrap(),
            })
        }
    }
}

#[derive(Portable)]
#[repr(C)]
pub struct ArchivedVersion {
    repr: ArchivedVersionRepr,
}

#[derive(Portable)]
#[repr(C)]
union ArchivedVersionRepr {
    inline: ArchivedU32,
    out_of_line: ManuallyDrop<RelPtr<ArchivedU64>>,
}

impl ArchivedVersion {
    pub fn is_inline(&self) -> bool {
        (unsafe { self.repr.inline } & ArchivedU32::from_native(u32::from(INLINE_MARKER))) != 0
    }

    pub fn get(&self) -> Version {
        if self.is_inline() {
            Version(unsafe { self.repr.inline.to_native().into() })
        } else {
            Version(unsafe { (*self.repr.out_of_line.as_ptr_wrapping()).to_native() })
        }
    }
}

unsafe impl<C> CheckBytes<C> for ArchivedVersion
where
    C: Fallible + ?Sized,
    C::Error: Source,
{
    unsafe fn check_bytes(value: *const Self, _: &mut C) -> Result<(), <C as Fallible>::Error> {
        let value = &*value;
        if value.is_inline() {
            if (value.repr.inline.to_native() & 0b10) != 0 {
                #[derive(Debug, thiserror::Error)]
                #[error("illegal bit {0} set in version")]
                struct IllegalBitError(usize);

                fail!(IllegalBitError(1))
            }
            Ok(())
        } else {
            if !value.repr.out_of_line.as_ptr_wrapping().is_aligned() {
                #[derive(Debug, thiserror::Error)]
                #[error("misaligned out-of-line pointer in version")]
                struct MisalignedError;

                fail!(MisalignedError)
            }
            Ok(())
        }
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
        fmt::Display::fmt(&self.get(), f)
    }
}

impl fmt::Debug for ArchivedVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Binary for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.0)
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
            assert_eq!(
                version.major(),
                major,
                "major version mismatch: {version:b}"
            );
            assert_eq!(
                version.minor(),
                minor,
                "minor version mismatch: {version:b}"
            );
            assert_eq!(
                version.patch(),
                patch,
                "patch version mismatch: {version:b}"
            );
        }
        case(0, 0, 0);
        case(1, 0, 0);
        case(1, 1, 1);
        case(1, 0, 1);
        case(69, 4, 2);
        case(31251241231, 0, 0);
        case(69, 201, 131125);
    }
}
