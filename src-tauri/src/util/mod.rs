pub mod http;
mod progress;
pub mod rkyv;
pub mod search;
pub mod serde;

#[cfg(windows)]
pub mod windows;

pub use progress::Progress;

use std::io;

pub trait IoErrorKindExt {
    fn is_not_found(&self) -> bool;
}

impl IoErrorKindExt for io::ErrorKind {
    fn is_not_found(&self) -> bool {
        matches!(self, io::ErrorKind::NotFound)
    }
}

impl IoErrorKindExt for io::Error {
    fn is_not_found(&self) -> bool {
        self.kind().is_not_found()
    }
}

macro_rules! hyphenated_uuid {
    ($id:expr) => {
        $id.hyphenated().encode_lower(&mut Uuid::encode_buffer())
    };
}
pub(crate) use hyphenated_uuid;

pub trait UsizeExt {
    fn as_u64(self) -> u64;
}

impl UsizeExt for usize {
    #[inline]
    fn as_u64(self) -> u64 {
        self.try_into()
            .expect("how is life with 128-bit computers?")
    }
}
