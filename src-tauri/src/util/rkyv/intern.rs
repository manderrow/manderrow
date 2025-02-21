use std::borrow::{Borrow, ToOwned};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::ops::Deref;

use rkyv::bytecheck::Verify;
use rkyv::munge::munge;
use rkyv::niche::niching::Niching;
use rkyv::primitive::FixedUsize;
use rkyv::rancor::{fail, Fallible, Source};
use rkyv::ser::sharing::SharingState;
use rkyv::ser::Writer;
use rkyv::string::{
    repr::{ArchivedStringRepr, INLINE_CAPACITY, OUT_OF_LINE_CAPACITY},
    ArchivedString, StringResolver,
};
use rkyv::validation::{ArchiveContext, SharedContext};
use rkyv::with::{ArchiveWith, SerializeWith};
use rkyv::{Place, SerializeUnsized};
use rkyv_intern::{Interning, InterningExt};

use super::FE;

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
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

impl Deref for ArchivedInternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.repr.as_str()
    }
}

impl<'a> rkyv::Archive for InternedString<'a> {
    type Archived = ArchivedInternedString;
    type Resolver = StringResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        StringIntern::resolve_with(self, resolver, out);
    }
}

impl<'a, S> rkyv::Serialize<S> for InternedString<'a>
where
    S: Interning<str> + Writer + Fallible + ?Sized,
    S::Error: Source,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        StringIntern::serialize_with(self, serializer)
    }
}

#[derive(rkyv::Portable, rkyv::bytecheck::CheckBytes)]
#[bytecheck(verify, crate = rkyv::bytecheck)]
#[repr(transparent)]
pub struct ArchivedInternedString {
    repr: ArchivedStringRepr,
}

unsafe impl<C> Verify<C> for ArchivedInternedString
where
    C: Fallible + ArchiveContext + SharedContext + ?Sized,
    C::Error: Source,
{
    fn verify(&self, context: &mut C) -> Result<(), C::Error> {
        use rkyv::bytecheck::CheckBytes;
        use rkyv::validation::shared::ValidationState;
        use rkyv::validation::ArchiveContextExt;
        if self.repr.is_inline() {
            unsafe {
                str::check_bytes(self.repr.as_str_ptr(), context)?;
            }
        } else {
            let base = (&self.repr as *const ArchivedStringRepr).cast::<u8>();
            let offset = unsafe { self.repr.out_of_line_offset() };
            let metadata = self.repr.len();

            let address = base.wrapping_offset(offset).cast::<()>();
            let ptr = rkyv::ptr_meta::from_raw_parts(address, metadata);

            let addr = ptr as *const u8 as usize;
            let type_id = std::any::TypeId::of::<ArchivedInternedString>();

            match context.start_shared(addr, type_id)? {
                ValidationState::Started => {
                    context.in_subtree(ptr, |context| {
                        // SAFETY: `in_subtree` has guaranteed that `ptr` is
                        // properly aligned and points to enough bytes to represent
                        // the pointed-to `str`.
                        unsafe { str::check_bytes(ptr, context) }
                    })?;
                    context.finish_shared(addr, type_id)?;
                }
                ValidationState::Pending => unreachable!(),
                ValidationState::Finished => (),
            }
        }

        Ok(())
    }
}

impl<'a> Niching<ArchivedInternedString> for FE {
    unsafe fn is_niched(niched: *const ArchivedInternedString) -> bool {
        let niched = niched.as_uninit_ref().unwrap();
        munge!(let ArchivedInternedString { repr } = niched);
        <FE as Niching<ArchivedString>>::is_niched(repr.as_ptr().cast())
    }

    fn resolve_niched(out: Place<ArchivedInternedString>) {
        munge!(let ArchivedInternedString { repr } = out);
        <FE as Niching<ArchivedString>>::resolve_niched(unsafe { repr.cast_unchecked() })
    }
}

#[derive(Debug)]
pub struct StringIntern;

impl<T: Deref<Target = str>> ArchiveWith<T> for StringIntern {
    type Archived = ArchivedInternedString;
    type Resolver = StringResolver;

    fn resolve_with(field: &T, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedString::resolve_from_str(field.deref(), resolver, unsafe { out.cast_unchecked() });
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
        let field = field.deref();
        if field.len() <= INLINE_CAPACITY || field.len() > OUT_OF_LINE_CAPACITY {
            ArchivedString::serialize_from_str(field, serializer)
        } else {
            Ok(unsafe {
                std::mem::transmute::<_, Self::Resolver>(
                    serializer.serialize_interned(field)? as FixedUsize
                )
            })
        }
    }
}

/// A general-purpose value interner.
pub struct Interner<T> {
    value_to_pos: HashMap<T, Option<NonZeroUsize>>,
}

impl<T> Interner<T> {
    /// Returns a new, empty interner.
    pub fn new() -> Self {
        Self {
            value_to_pos: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.value_to_pos.len()
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.value_to_pos.keys()
    }
}

impl<T> Default for Interner<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct NotStarted;

impl fmt::Display for NotStarted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "value was not started interning")
    }
}

impl Error for NotStarted {}

#[derive(Debug)]
struct AlreadyFinished;

impl fmt::Display for AlreadyFinished {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "value was already finished interning")
    }
}

impl Error for AlreadyFinished {}

impl<T, E> Interning<T, E> for Interner<T::Owned>
where
    T::Owned: Hash + Eq + Borrow<T>,
    T: Hash + Eq + ToOwned + ?Sized,
    E: Source,
{
    fn start_interning(&mut self, value: &T) -> SharingState {
        match self.value_to_pos.get(value) {
            None => {
                self.value_to_pos.insert(value.to_owned(), None);
                SharingState::Started
            }
            Some(None) => SharingState::Pending,
            Some(Some(pos)) => SharingState::Finished(pos.get() - 1),
        }
    }

    fn finish_interning(&mut self, value: &T, pos: usize) -> Result<(), E> {
        match self.value_to_pos.get_mut(value) {
            None => fail!(NotStarted),
            Some(Some(_)) => fail!(AlreadyFinished),
            Some(x) => {
                *x = Some(NonZeroUsize::new(pos + 1).unwrap());
                Ok(())
            }
        }
    }
}
