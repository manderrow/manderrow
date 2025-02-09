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
