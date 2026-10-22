use std::{
    mem::ManuallyDrop,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
#[error("Failed to lock directory {path}: {error}")]
pub struct DirLockError {
    path: PathBuf,
    error: std::io::Error,
}

pub struct DirLock {
    // exists only to keep the lock active
    #[allow(unused)]
    lock_file: ManuallyDrop<std::fs::File>,
    lock_path: PathBuf,
}

impl DirLock {
    pub async fn lock_dir(path: impl AsRef<Path>, shared: bool) -> Result<Self, DirLockError> {
        let path = path.as_ref();
        let lock_path = path.join(".~lock");
        let file = tokio::fs::File::create(&lock_path)
            .await
            .map_err(|e| DirLockError {
                path: path.to_owned(),
                error: e,
            })?;
        let lock_file = file.into_std().await;
        if shared {
            lock_file.lock_shared()
        } else {
            lock_file.lock()
        }
        .map_err(|e| DirLockError {
            path: path.to_owned(),
            error: e,
        })?;
        Ok(Self {
            lock_file: ManuallyDrop::new(lock_file),
            lock_path,
        })
    }

    pub fn into_file(self) -> std::fs::File {
        let mut this = ManuallyDrop::new(self);
        // SAFETY: `this` is never dropped
        unsafe { ManuallyDrop::take(&mut this.lock_file) }
    }
}

impl Drop for DirLock {
    fn drop(&mut self) {
        // SAFETY: called in drop, never called more than once
        unsafe { ManuallyDrop::drop(&mut self.lock_file) };
        if let Err(e) = std::fs::remove_file(&self.lock_path) {
            slog_scope::error!("Failed to remove lock file {:?}: {e}", self.lock_path);
        }
    }
}
