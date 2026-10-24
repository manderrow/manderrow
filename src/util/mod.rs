pub mod http;
pub mod http_cache;
mod progress;
pub mod search;

pub use progress::Progress;

use std::io;
use std::path::PathBuf;

/// Returns the memory that this thread has freed to the OS. The allocator otherwise keeps it until
/// the thread allocates again, which after decoding a mod index can be hundreds of megabytes.
pub fn release_freed_memory() {
    // SAFETY: this takes no pointers, and only returns memory that is no longer allocated
    unsafe { libmimalloc_sys::mi_collect(true) }
}

/// Returns the memory that Qt and the other native libraries have freed to the OS. They allocate
/// from glibc's allocator, which by itself only returns what is at the end of its heaps, and keeps
/// what is freed between memory still in use, such as the images of a mod's README after it's
/// closed. The system allocators of Windows and macOS return such memory by themselves.
pub fn trim_native_heap() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> std::ffi::c_int;
        }
        // SAFETY: this takes no pointers, and only returns memory that is no longer allocated
        unsafe { malloc_trim(0) };
    }
}

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

/// Returns the path to start the app from, which is the AppImage when running from one. The
/// executable is otherwise inside the AppImage's mount, which changes with each run and is gone
/// once the app exits.
pub fn launcher_path() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    if cfg!(target_os = "linux") {
        // The AppImage runtime sets these, which programs started from another AppImage inherit.
        if let (Some(appimage), Some(appdir)) =
            (std::env::var_os("APPIMAGE"), std::env::var_os("APPDIR"))
        {
            if exe.starts_with(appdir) {
                return Ok(appimage.into());
            }
        }
    }
    Ok(exe)
}
