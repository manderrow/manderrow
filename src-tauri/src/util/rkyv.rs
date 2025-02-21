mod intern;

pub use intern::{ArchivedInternedString, InternedString, StringIntern};

use rkyv::niche::niching::Niching;
use rkyv::rancor::{Fallible, Source};
use rkyv::string::{ArchivedString, StringResolver};
use rkyv::with::{ArchiveWith, SerializeWith};
use rkyv::{Place, SerializeUnsized};

pub struct InlineAsString;

impl ArchiveWith<&str> for InlineAsString {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve_with(field: &&str, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedString::resolve_from_str(*field, resolver, out);
    }
}

impl<S> SerializeWith<&str, S> for InlineAsString
where
    S: Fallible + ?Sized,
    S::Error: Source,
    str: SerializeUnsized<S>,
{
    fn serialize_with(field: &&str, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(*field, serializer)
    }
}

/// Niche value begins with a `0xfe` byte.
pub struct FE;

impl Niching<ArchivedString> for FE {
    unsafe fn is_niched(niched: *const ArchivedString) -> bool {
        niched.cast::<u8>().read() == 0xfe
    }

    fn resolve_niched(out: Place<ArchivedString>) {
        let out = unsafe { out.cast_unchecked::<[u8; rkyv::string::repr::INLINE_CAPACITY]>() };
        out.write([0xfe; rkyv::string::repr::INLINE_CAPACITY]);
    }
}
