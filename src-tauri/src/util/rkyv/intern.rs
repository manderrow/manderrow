use std::any::TypeId;
use std::ops::Deref;

use rkyv::bytecheck::Verify;
use rkyv::munge::munge;
use rkyv::niche::niching::{Niching, Null};
use rkyv::primitive::{ArchivedUsize, FixedUsize};
use rkyv::rancor::{Fallible, Source};
use rkyv::ser::{Writer, WriterExt};
use rkyv::validation::{ArchiveContext, SharedContext};
use rkyv::with::{ArchiveWith, SerializeWith};
use rkyv::{Archive, Place, RelPtr, Serialize, SerializeUnsized};
use rkyv_intern::{Interning, InterningState};

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
        munge!(let Self::Archived { ptr } = out);
        RelPtr::emplace_unsized(resolver.pos.try_into().unwrap(), (), ptr);
    }
}

impl<'a, S> Serialize<S> for InternedString<'a>
where
    S: Interning<str> + Writer + Fallible + ?Sized,
    S::Error: Source,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
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

// TODO: inline repr for <= 4 bytes
#[derive(rkyv::Portable, rkyv::bytecheck::CheckBytes)]
#[bytecheck(verify, crate = rkyv::bytecheck)]
#[repr(C)]
pub struct ArchivedInternedString {
    ptr: RelPtr<ArchivedInternedStringHeader>,
}

impl ArchivedInternedString {
    pub fn len(&self) -> usize {
        unsafe { usize::try_from((*self.ptr.as_ptr_wrapping()).len.to_native()).unwrap() }
    }

    pub fn as_str_ptr(&self) -> *const str {
        rkyv::ptr_meta::from_raw_parts(
            unsafe { self.ptr.as_ptr_wrapping().add(1).cast() },
            self.len(),
        )
    }
}

#[derive(rkyv::Portable, rkyv::bytecheck::CheckBytes)]
#[bytecheck(crate = rkyv::bytecheck)]
#[repr(C)]
pub struct ArchivedInternedStringHeader {
    len: ArchivedUsize,
}

pub type InternedStringNiche = Null;

impl<'a> Niching<ArchivedInternedString> for Null {
    unsafe fn is_niched(niched: *const ArchivedInternedString) -> bool {
        let niched = niched.as_uninit_ref().unwrap();
        munge!(let ArchivedInternedString { ptr } = niched);
        (*ptr.as_ptr().cast::<RelPtr<str>>()).is_invalid()
    }

    fn resolve_niched(out: Place<ArchivedInternedString>) {
        munge!(let ArchivedInternedString { ptr } = out);
        RelPtr::emplace_invalid(ptr);
    }
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
        let ptr = self.ptr.as_ptr_wrapping();
        let type_id = TypeId::of::<ArchivedInternedString>();

        match context.start_shared(ptr.addr(), type_id)? {
            ValidationState::Started => {
                context.check_subtree_ptr(
                    ptr.cast(),
                    &std::alloc::Layout::new::<ArchivedInternedStringHeader>(),
                )?;

                let str_ptr = self.as_str_ptr();
                context.in_subtree(str_ptr, |context| unsafe {
                    str::check_bytes(str_ptr, context)
                })?;
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
