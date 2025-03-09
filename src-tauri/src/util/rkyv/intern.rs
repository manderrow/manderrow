use std::any::TypeId;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::ptr::addr_of;

use rkyv::bytecheck::{CheckBytes, Verify};
use rkyv::munge::munge;
use rkyv::niche::niching::Niching;
use rkyv::primitive::{ArchivedIsize, ArchivedUsize, FixedIsize, FixedUsize};
use rkyv::rancor::{Fallible, Panic, ResultExt, Source};
use rkyv::rel_ptr::Offset;
use rkyv::ser::{Writer, WriterExt};
use rkyv::validation::{ArchiveContext, SharedContext};
use rkyv::with::{ArchiveWith, SerializeWith};
use rkyv::{Archive, Place, Serialize, SerializeUnsized};
use rkyv_intern::{Interning, InterningState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct InternedString<'a>(pub &'a str);

impl<'a> From<&'a str> for InternedString<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl<'a> Deref for InternedString<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> AsRef<str> for InternedString<'a> {
    fn as_ref(&self) -> &str {
        self
    }
}

impl Deref for ArchivedInternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_str_ptr() }
    }
}

impl<'a> From<&'a ArchivedInternedString> for InternedString<'a> {
    fn from(value: &'a ArchivedInternedString) -> Self {
        Self(value)
    }
}

pub struct InternedStringResolver {
    pos: FixedUsize,
}

impl<'a> Archive for InternedString<'a> {
    type Archived = ArchivedInternedString;
    type Resolver = InternedStringResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let Self::Archived { repr } = out);
        if self.len() <= INLINE_CAPACITY {
            debug_assert_eq!(resolver.pos, 0);
            munge!(let ArchivedInternedStringRepr { inline } = repr);
            let inline = unsafe { inline.cast_unchecked::<Inline>() };
            munge!(let Inline { data } = inline);
            let dst = unsafe { data.ptr().cast::<u8>() };
            unsafe { dst.copy_from_nonoverlapping(self.0.as_ptr(), self.len()) };
            if self.len() < INLINE_CAPACITY {
                unsafe { dst.add(self.len()).write(END_MARKER) };
            }
        } else {
            munge!(let ArchivedInternedStringRepr { out_of_line } = repr);
            let out_of_line = unsafe { out_of_line.cast_unchecked::<OutOfLine>() };
            munge!(let OutOfLine { value: out } = out_of_line);
            let offset = ArchivedIsize::from_isize::<Panic>(
                rkyv::rel_ptr::signed_offset::<Panic>(out.pos(), resolver.pos.try_into().unwrap())
                    .always_ok(),
            )
            .always_ok();
            let value = offset.to_native() as u32;
            debug_assert_eq!(value & 0b11, 0, "unaligned offset");
            let value = (value & 0xffffff3c) | u32::from(PTR_MARKER) | ((value & 0xc0) >> 6);
            out.write(value.into());
        }
    }
}

impl<'a, S> Serialize<S> for InternedString<'a>
where
    S: Interning<str> + Writer + Fallible + ?Sized,
    S::Error: Source,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        if self.len() <= INLINE_CAPACITY {
            Ok(InternedStringResolver { pos: 0 })
        } else {
            let pos = match serializer.start_interning(self.0) {
                InterningState::Started(state) => {
                    let pos = {
                        let mut header = ArchivedInternedStringHeader {
                            len: ArchivedUsize::from_native(
                                FixedUsize::try_from(self.0.len()).map_err(S::Error::new)?,
                            ),
                        };

                        serializer.align_for::<ArchivedInternedStringHeader>()?;
                        let pos = serializer.pos();
                        let out = unsafe { Place::new_unchecked(pos, &mut header) };
                        serializer.write(out.as_slice())?;
                        pos
                    };
                    let pos2 = self.0.serialize_unsized(serializer)?;
                    assert_eq!(
                        pos2,
                        pos + size_of::<ArchivedInternedStringHeader>(),
                        "unexpected data offset"
                    );
                    serializer.finish_interning(state, pos)?;
                    pos
                }
                InterningState::Pending => {
                    #[derive(Debug, thiserror::Error)]
                    #[error("encountered cyclic shared pointers while interning")]
                    struct CyclicInternedValueError;

                    rkyv::rancor::fail!(CyclicInternedValueError)
                }
                InterningState::Finished(pos) => pos,
            };
            Ok(InternedStringResolver {
                pos: pos as FixedUsize,
            })
        }
    }
}

#[derive(rkyv::Portable, rkyv::bytecheck::CheckBytes)]
#[bytecheck(crate = rkyv::bytecheck)]
#[repr(C)]
pub struct ArchivedInternedString {
    repr: ArchivedInternedStringRepr,
}

impl ArchivedInternedString {
    pub fn len(&self) -> usize {
        if self.repr.is_inline() {
            unsafe { self.repr.inline.len() }
        } else {
            unsafe { self.repr.out_of_line.len() }
        }
    }

    pub fn as_str_ptr(&self) -> *const str {
        if self.repr.is_inline() {
            unsafe { self.repr.inline.as_str_ptr() }
        } else {
            unsafe { self.repr.out_of_line.as_str_ptr() }
        }
    }
}

const INLINE_CAPACITY: usize = size_of::<OutOfLine>();

const PTR_MARKER_MASK: u8 = 0xc0;

const PTR_MARKER: u8 = 0x80;
// could use PTR_MARKER with null offset instead
const NICHE_MARKER: u8 = 0xc0;

const END_MARKER: u8 = 0xff;

#[derive(rkyv::Portable)]
#[repr(C, align(4))]
union ArchivedInternedStringRepr {
    inline: ManuallyDrop<Inline>,
    out_of_line: ManuallyDrop<OutOfLine>,
}

impl ArchivedInternedStringRepr {
    pub fn is_inline(&self) -> bool {
        unsafe { self.inline.data[0] & PTR_MARKER_MASK != PTR_MARKER }
    }
}

#[derive(rkyv::Portable, CheckBytes)]
#[bytecheck(verify, crate = rkyv::bytecheck)]
#[repr(C, align(4))]
struct Inline {
    data: [u8; INLINE_CAPACITY],
}

impl Inline {
    pub fn len(&self) -> usize {
        self.data
            .iter()
            .position(|&b| b == 0xff)
            .unwrap_or(self.data.len())
    }

    pub fn as_str_ptr(&self) -> *const str {
        rkyv::ptr_meta::from_raw_parts(self.data.as_ptr().cast(), self.len())
    }
}

#[derive(rkyv::Portable, CheckBytes)]
#[bytecheck(verify, crate = rkyv::bytecheck)]
#[repr(C, align(4))]
struct OutOfLine {
    /// The 8th and 7th bits are reserved for the PTR_MARKER.
    /// The original values of those two bits is moved to the least significant
    /// two bits, whose original value will be 00 thanks to alignment.
    value: ArchivedUsize,
}

impl OutOfLine {
    pub fn offset(&self) -> FixedIsize {
        let value = self.value.to_native();
        debug_assert_eq!(value & 0xc0, u32::from(PTR_MARKER), "PTR_MARKER missing");
        ((value & 0xffffff3c) | ((value & 0b11) << 6)) as FixedIsize
    }

    pub fn as_ptr(&self) -> *const ArchivedInternedStringHeader {
        (self as *const Self)
            .cast::<u8>()
            .wrapping_offset(self.offset().try_into().unwrap())
            .cast()
    }

    pub fn len(&self) -> usize {
        unsafe { usize::try_from((*self.as_ptr()).len.to_native()).unwrap() }
    }

    pub fn as_str_ptr(&self) -> *const str {
        rkyv::ptr_meta::from_raw_parts(unsafe { self.as_ptr().add(1).cast() }, self.len())
    }
}

unsafe impl<C> CheckBytes<C> for ArchivedInternedStringRepr
where
    C: Fallible + ArchiveContext + SharedContext + ?Sized,
    C::Error: Source,
{
    unsafe fn check_bytes(value: *const Self, context: &mut C) -> Result<(), C::Error> {
        if (*value).is_inline() {
            let inline = addr_of!((*value).inline);
            Inline::check_bytes(inline.cast(), context)
                .trace("while checking inline interned string")
        } else {
            let out_of_line = addr_of!((*value).out_of_line);
            OutOfLine::check_bytes(out_of_line.cast(), context)
                .trace("while checking out-of-line interned string")
        }
    }
}

#[derive(rkyv::Portable, rkyv::bytecheck::CheckBytes)]
#[bytecheck(crate = rkyv::bytecheck)]
#[repr(C, align(4))]
struct ArchivedInternedStringHeader {
    len: ArchivedUsize,
}

pub struct InternedStringNiche;

impl<'a> Niching<ArchivedInternedString> for InternedStringNiche {
    unsafe fn is_niched(niched: *const ArchivedInternedString) -> bool {
        let niched = niched.as_uninit_ref().unwrap();
        munge!(let ArchivedInternedString { repr: ArchivedInternedStringRepr { inline } } = niched);
        *inline.as_ptr().cast::<u8>() == NICHE_MARKER
    }

    fn resolve_niched(out: Place<ArchivedInternedString>) {
        munge!(let ArchivedInternedString { repr } = out);
        unsafe {
            repr.write_unchecked(ArchivedInternedStringRepr {
                inline: ManuallyDrop::new(Inline {
                    data: [NICHE_MARKER, 0, 0, 0],
                }),
            })
        };
    }
}

unsafe impl<C> Verify<C> for Inline
where
    C: Fallible + ?Sized,
    C::Error: Source,
{
    fn verify(&self, context: &mut C) -> Result<(), C::Error> {
        unsafe { str::check_bytes(self.as_str_ptr(), context) }
    }
}

unsafe impl<C> Verify<C> for OutOfLine
where
    C: Fallible + ArchiveContext + SharedContext + ?Sized,
    C::Error: Source,
{
    fn verify(&self, context: &mut C) -> Result<(), <C as Fallible>::Error> {
        use rkyv::validation::shared::ValidationState;
        use rkyv::validation::ArchiveContextExt;

        if self.offset() == 0 {
            #[derive(Debug, thiserror::Error)]
            #[error("encountered zero offset while validating")]
            struct ZeroOffsetError;

            rkyv::rancor::fail!(ZeroOffsetError)
        }

        let ptr = self.as_ptr();
        let type_id = TypeId::of::<ArchivedInternedString>();

        match context.start_shared(ptr.addr(), type_id)? {
            ValidationState::Started => {
                context
                    .check_subtree_ptr(
                        ptr.cast(),
                        &std::alloc::Layout::new::<ArchivedInternedStringHeader>(),
                    )
                    .trace("while checking header")?;

                let str_ptr = self.as_str_ptr();
                context
                    .in_subtree(str_ptr, |context| unsafe {
                        str::check_bytes(str_ptr, context)
                    })
                    .trace("while checking data")?;
                context.finish_shared(ptr.addr(), type_id)?;
            }
            ValidationState::Pending => {
                #[derive(Debug, thiserror::Error)]
                #[error("encountered cyclic shared pointers while validating")]
                struct CyclicSharedPointerError;

                rkyv::rancor::fail!(CyclicSharedPointerError)
            }
            ValidationState::Finished => {}
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct StringIntern;

impl<T: Deref<Target = str>> ArchiveWith<T> for StringIntern {
    type Archived = ArchivedInternedString;
    type Resolver = InternedStringResolver;

    fn resolve_with(field: &T, resolver: Self::Resolver, out: Place<Self::Archived>) {
        InternedString(field.deref()).resolve(resolver, out)
    }
}

impl<T, S> SerializeWith<T, S> for StringIntern
where
    T: Deref<Target = str>,
    T::Target: SerializeUnsized<S>,
    S: Interning<T::Target> + Writer + Fallible + ?Sized,
    S::Error: Source,
{
    fn serialize_with(
        field: &T,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        InternedString(field.deref()).serialize(serializer)
    }
}
