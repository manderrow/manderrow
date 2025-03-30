//! A restricted variant of SemVer that supports only `MAJOR.MINOR.PATCH` with up to 16 total characters.

use std::{
    fmt::{self, Formatter},
    mem::ManuallyDrop,
    num::ParseIntError,
};

#[cfg(feature = "statistics")]
use std::sync::atomic::AtomicU32;

use rkyv::{
    Portable, RelPtr, SerializeUnsized,
    bytecheck::CheckBytes,
    munge::munge,
    primitive::{ArchivedU32, ArchivedU64, FixedUsize},
    rancor::{Fallible, Source, fail},
    ser::Writer,
};

/// Returns the minimum number of bits required to store the value.
fn bit_len(value: u64) -> u32 {
    u64::BITS - value.leading_zeros()
}

#[derive(Debug, thiserror::Error)]
#[error("Too many bits in version")]
pub struct TooManyBitsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Components {
    digits: u64,
    minor_exp_m1: u32,
    major_exp_m1: u32,
}

impl Components {
    #[inline]
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        let minor_exp_m1 = patch.checked_ilog10().unwrap_or(0);
        let major_exp_m1 = minor.checked_ilog10().unwrap_or(0);

        let minor_mul = EXP_LUT[minor_exp_m1 as usize];
        let major_mul = EXP_LUT[major_exp_m1 as usize];

        Self {
            digits: patch + minor * minor_mul + major * minor_mul * major_mul,
            minor_exp_m1,
            major_exp_m1,
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct Packer {
    pub digit_bits: u32,
    pub index_bits: u32,
    pub digit_mask: u64,
    pub index_mask: u8,
}

impl Packer {
    #[inline]
    fn pack(&self, components: Components) -> Result<u64, TooManyBitsError> {
        let Components {
            digits,
            minor_exp_m1,
            major_exp_m1,
        } = components;

        if bit_len(digits) > self.digit_bits {
            return Err(TooManyBitsError);
        }

        debug_assert!(bit_len(minor_exp_m1.into()) <= self.index_bits);
        debug_assert!(bit_len(major_exp_m1.into()) <= self.index_bits);

        Ok(digits << 2
            | (u64::from(major_exp_m1) << (2 + self.digit_bits + self.index_bits))
            | (u64::from(minor_exp_m1) << (2 + self.digit_bits)))
    }

    #[inline]
    pub fn unpack(self, value: u64) -> (u64, u64, u64) {
        // discard marker
        let value = value >> 2;

        let minor_exp_m1 = (((value >> self.digit_bits) as u8) & self.index_mask) as usize;
        let major_exp_m1 =
            (((value >> (self.digit_bits + self.index_bits)) as u8) & self.index_mask) as usize;
        let digits = value & self.digit_mask;

        let minor_mul = EXP_LUT[minor_exp_m1];
        let major_mul = EXP_LUT[major_exp_m1];

        (
            digits / minor_mul / major_mul,
            (digits / minor_mul) % major_mul,
            digits % minor_mul,
        )
    }

    pub fn from_digits(n: u32) -> Self {
        // base10 packing with bit shifting and base10 "indices" (10^i)
        let digit_bits = ((n as f64) / 2.0f64.log10()).ceil() as u32;
        let index_bits = (n as f64).log2().ceil() as u32;
        Self {
            digit_bits,
            index_bits,
            digit_mask: !0u64 >> (64 - digit_bits),
            index_mask: !0u8 >> (8 - index_bits),
        }
    }
}

/// Powers of 10, starting from 1, up to and including [`Version::MAX_COMPONENT_DIGITS`].
const EXP_LUT: [u64; Version::MAX_COMPONENT_DIGITS as usize] = [
    10,
    100,
    1_000,
    10_000,
    100_000,
    1_000_000,
    10_000_000,
    100_000_000,
    1_000_000_000,
    10_000_000_000,
    100_000_000_000,
    1_000_000_000_000,
];

const INLINE_MARKER: u8 = 0b01;

const INLINE_PACKER: Packer = Packer {
    digit_bits: 24,
    index_bits: 3,
    digit_mask: 0xff_ff_ff,
    index_mask: 0b111,
};

const OUT_OF_LINE_PACKER: Packer = Packer {
    digit_bits: 47,
    index_bits: 4,
    digit_mask: 0x7f_ff_ff_ff_ff_ff,
    index_mask: 0b1111,
};

/// See https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/consts.py#L4 for some format information.
///
/// The [`PartialEq`], [`Eq`], and [`Hash`] trait impls rely on there being a single canonical representation that is always used for a given version.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version(u64);

impl Version {
    /// According to https://github.com/thunderstore-io/Thunderstore/blob/a4146daa5db13344be647a87f0206c1eb19eb90e/django/thunderstore/repository/models/package_version.py#L101-L103.
    const MAX_LEN: u32 = 16;
    /// [`MAX_LEN`] minus 2 for the separators.
    const MAX_TOTAL_DIGITS: u32 = Self::MAX_LEN - 2;
    /// Each component must have at least one digit, thus we subtract two from the total
    /// number of digits to find the maximum digits for each component.
    const MAX_COMPONENT_DIGITS: u32 = Self::MAX_TOTAL_DIGITS - 2;

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
        let value = unsafe { &*value };
        if value.is_inline() {
            if (unsafe { value.repr.inline }.to_native() & 0b10) != 0 {
                #[derive(Debug, thiserror::Error)]
                #[error("illegal bit {0} set in version")]
                struct IllegalBitError(usize);

                fail!(IllegalBitError(1))
            }
            Ok(())
        } else {
            if !unsafe { value.repr.out_of_line.as_ptr_wrapping() }.is_aligned() {
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
    use super::{Components, Version};

    #[test]
    fn test_packing_roundtrip() {
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
        case(999_999_999_999, 9, 9);
    }

    #[test]
    fn test_calculations() {
        // base2 packing with bit shifting and bit indices
        let max_bits =
            ((((Version::MAX_TOTAL_DIGITS as f64) / 3.0) / 2.0f64.log10()).ceil() as u32) * 3;
        let index_bits = (max_bits - 1).next_power_of_two().ilog2() + 1;

        assert_eq!(max_bits, 48);
        assert_eq!(index_bits, 7);
        assert_eq!(max_bits + index_bits * 2, 62);

        // base10 packing with bit shifting and base10 "indices" (10^i)
        let max_bits = ((Version::MAX_TOTAL_DIGITS as f64) / 2.0f64.log10()).ceil() as u32;
        let index_bits = (Version::MAX_TOTAL_DIGITS as f64).log2().ceil() as u32;
        assert_eq!(max_bits, 47);
        assert_eq!(index_bits, 4);
        assert_eq!(max_bits + index_bits * 2, 55);
    }

    #[test]
    fn test_components() {
        assert_eq!(
            Components::new(999_999_999_999, 9, 9),
            Components {
                digits: 999_999_999_999_9_9,
                minor_exp_m1: 0,
                major_exp_m1: 0,
            }
        );

        assert_eq!(
            Components::new(9, 9, 999_999_999_999),
            Components {
                digits: 9_9_999_999_999_999,
                minor_exp_m1: 11,
                major_exp_m1: 0,
            }
        );
    }
}
