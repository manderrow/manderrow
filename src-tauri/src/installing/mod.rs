//! Package installation helpers
//!
//! Never make changes to `IndexEntryV*` or [`Index`] variants. Make a new version instead.

pub mod commands;
mod index;

use std::ffi::OsString;
use std::io::Write;
use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use base64::Engine;
use bytes::{Bytes, BytesMut};
use fs4::tokio::AsyncFileExt;
use index::{ArchivedIndex, ArchivedIndexEntryV1, Index, IndexEntryRef, IndexEntryV1, IndexPath};
use manderrow_paths::cache_dir;
use slog::{debug, trace};
use tauri::AppHandle;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use trie_rs::TrieBuilder;
use walkdir::WalkDir;
use zip::{result::ZipError, ZipArchive};

use crate::tasks::{self, TaskBuilder, TaskHandle};
use crate::util::{IoErrorKindExt, UsizeExt};
use crate::Reqwest;

const INDEX_FILE_NAME: &str = ".manderrow_content_index";

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Status {
    /// A file had its content modified from that which came with the package.
    ContentModified,
    /// A file or directory that did not come with the package was created.
    Created,
    /// A filesystem object was replaced with one of a different type, or a symlink's target changed.
    TypeChanged,
    /// A symlink's target changed.
    LinkTargetChanged,
    /// A filesystem object that came with the package was deleted.
    Deleted,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("No package index was found")]
    IndexNotFoundError,

    #[error("{0}")]
    IoError(#[from] std::io::Error, std::backtrace::Backtrace),

    #[error(transparent)]
    WalkDirError(#[from] walkdir::Error),

    #[error("Unable to read package content index: {0}")]
    ReadIndexError(#[source] std::io::Error),

    #[error("Invalid package content index: {0}")]
    InvalidIndexError(#[source] rkyv::rancor::Error),

    #[error("Internal error: {0}")]
    Internal(#[source] anyhow::Error),
}

fn hash_file(path: &Path) -> std::io::Result<blake3::Hash> {
    Ok(blake3::Hasher::new().update_mmap(&path)?.finalize())
}

pub async fn scan_installed_package_for_changes<'i>(
    log: &slog::Logger,
    path: &Path,
    buf: &mut impl Extend<(PathBuf, Status)>,
) -> Result<(), ScanError> {
    let mut index_buf = Vec::new();
    scan_installed_package_for_changes_with_index_buf(log, path, buf, &mut index_buf).await?;
    Ok(())
}

async fn scan_installed_package_for_changes_with_index_buf<'i>(
    log: &slog::Logger,
    path: &Path,
    buf: &mut impl Extend<(PathBuf, Status)>,
    index_buf: &'i mut Vec<u8>,
) -> Result<Option<&'i ArchivedIndex>, ScanError> {
    match tokio::fs::metadata(&path).await {
        Ok(m) if m.is_dir() => {}
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Path exists, but is not a directory",
            )
            .into())
        }
        Err(e) if e.is_not_found() => return Err(ScanError::IndexNotFoundError),
        Err(e) => return Err(e.into()),
    };
    let index_buf = match tokio::fs::File::open(path.join(INDEX_FILE_NAME)).await {
        Ok(mut f) => {
            f.read_to_end(index_buf)
                .await
                .map_err(ScanError::ReadIndexError)?;
            Some(index_buf)
        }
        Err(e) if e.is_not_found() => None,
        Err(e) => return Err(ScanError::ReadIndexError(e)),
    };
    let index = match index_buf {
        Some(index_buf) => Some(
            rkyv::access::<ArchivedIndex, rkyv::rancor::Error>(index_buf)
                .map_err(ScanError::InvalidIndexError)?,
        ),
        None => None,
    };

    let mut iter = WalkDir::new(path).into_iter();
    if iter
        .next()
        .context("Expected root entry")
        .map_err(ScanError::Internal)??
        .path()
        != path
    {
        return Err(ScanError::Internal(anyhow!("First entry was not root")));
    }
    while let Some(r) = iter.next() {
        let dir_entry = r?;
        let rel_path = dir_entry
            .path()
            .strip_prefix(path)
            .map_err(|e| ScanError::Internal(e.into()))?;
        if rel_path == Path::new(INDEX_FILE_NAME) {
            continue;
        }
        if let Some(entry) = index.and_then(|index| index.get(&rel_path)) {
            match entry {
                IndexEntryRef::V1(ArchivedIndexEntryV1::File { hash }) => {
                    let hash = blake3::Hash::from_bytes(*hash);
                    if !dir_entry.file_type().is_file() {
                        if dir_entry.file_type().is_dir() {
                            // new directory, don't create an entry for each child
                            iter.skip_current_dir();
                        }
                        buf.extend_one((dir_entry.path().to_owned(), Status::TypeChanged));
                    } else if tokio::task::block_in_place(|| hash_file(dir_entry.path()))? != hash {
                        buf.extend_one((dir_entry.path().to_owned(), Status::ContentModified))
                    }
                }
                IndexEntryRef::V1(ArchivedIndexEntryV1::Symlink { target }) => {
                    match tokio::fs::read_link(dir_entry.path()).await {
                        Ok(real_target) => {
                            let target = Path::new(target.as_str());
                            let real_target = if target.is_relative() {
                                if let Ok(real_target) = real_target.strip_prefix(path) {
                                    real_target
                                } else {
                                    &real_target
                                }
                            } else {
                                &real_target
                            };
                            if real_target == target {
                                buf.extend_one((
                                    dir_entry.path().to_owned(),
                                    Status::LinkTargetChanged,
                                ));
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
                            if dir_entry.file_type().is_dir() {
                                // new directory, don't create an entry for each child
                                iter.skip_current_dir();
                            }
                            buf.extend_one((dir_entry.path().to_owned(), Status::TypeChanged));
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                IndexEntryRef::V1(ArchivedIndexEntryV1::Directory) => {
                    if !dir_entry.file_type().is_dir() {
                        buf.extend_one((dir_entry.path().to_owned(), Status::TypeChanged));
                    }
                }
            }
        } else {
            if dir_entry.file_type().is_dir() {
                // new directory, don't create an entry for each child
                iter.skip_current_dir();
            }
            buf.extend_one((dir_entry.path().to_owned(), Status::Created));
        }
    }

    match index {
        Some(ArchivedIndex::V1(entries)) => {
            // TODO: remove collect when https://github.com/rkyv/rkyv/issues/578 is fixed
            for e_path in entries.iter().map(|(p, _)| p).collect::<Vec<_>>() {
                let mut p = path.to_owned();
                for comp in &*e_path.0 {
                    p.push(comp.as_str());
                }
                if !tokio::fs::try_exists(&p).await? {
                    // skip recording if a parent has been deleted.
                    if let Some((entry, _)) = entries.iter().find(|(e_p, _)| {
                        e_p.0.len() >= p.components().count()
                            && e_p.0.iter().zip(p.components()).all(|(a, b)| {
                                b.as_os_str()
                                    .to_str()
                                    .map(|b| a.as_str() == b)
                                    .unwrap_or(false)
                            })
                    }) {
                        trace!(log, "Not recording deletion because a parent was also deleted: {e_path:?} is inside of {entry:?}");
                    } else {
                        buf.extend_one((p, Status::Deleted));
                    }
                }
            }
        }
        Some(ArchivedIndex::V2(entries)) => {
            // TODO: remove collect when https://github.com/rkyv/rkyv/issues/578 is fixed
            for indexed_path in entries.iter().map(|(p, _)| p).collect::<Vec<_>>() {
                let mut p: PathBuf = path.to_owned();
                for comp in indexed_path.components() {
                    match comp {
                        Cow::Borrowed(comp) => p.push(comp),
                        Cow::Owned(comp) => p.push(comp),
                    }
                }
                if !tokio::fs::try_exists(&p).await? {
                    // skip recording if a parent has been deleted.
                    if let Some((entry, _)) = entries.iter().find(|(e_p, _)| {
                        e_p.component_count() >= p.components().count()
                            && e_p.components().zip(p.components()).all(|(a, b)| {
                                b.as_os_str().to_str().map(|b| &*a == b).unwrap_or(false)
                            })
                    }) {
                        trace!(log, "Not recording deletion because a parent was also deleted: {indexed_path:?} is inside of {entry:?}");
                    } else {
                        buf.extend_one((p, Status::Deleted));
                    }
                }
            }
        }
        None => {}
    }

    trace!(log, "Index: {index:#?}");

    Ok(index)
}

async fn generate_package_index(log: &slog::Logger, path: &Path) -> Result<()> {
    debug!(log, "Generating package index for {path:?}");

    let mut buf = HashMap::new();
    let mut iter = WalkDir::new(path).into_iter();
    ensure!(
        iter.next().context("Expected root entry")??.path() == path,
        "First entry was not root"
    );
    while let Some(r) = iter.next() {
        let e = r?;
        let rel_path = e.path().strip_prefix(path)?;
        let index_path = IndexPath::try_from(rel_path)?;
        let metadata = tokio::fs::symlink_metadata(e.path()).await?;
        let entry = if metadata.is_file() {
            IndexEntryV1::File {
                hash: tokio::task::block_in_place(|| hash_file(e.path()))?.into(),
            }
        } else if metadata.is_dir() {
            IndexEntryV1::Directory
        } else if metadata.is_symlink() {
            let target = tokio::fs::read_link(e.path()).await?;
            let target = if let Ok(rel_target) = target.strip_prefix(path) {
                rel_target.to_owned()
            } else {
                target
            };
            IndexEntryV1::Symlink {
                target: target
                    .into_os_string()
                    .into_string()
                    .map_err(|s| anyhow!("Unsupported path in zip archive: {s:?}"))?,
            }
        } else {
            bail!(
                "Unsupported file type in newly extracted package directory: {:?}",
                metadata.file_type()
            )
        };
        buf.insert(index_path, entry);
    }
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&Index::V1(buf))?;
    tokio::fs::write(path.join(INDEX_FILE_NAME), bytes).await?;
    Ok(())
}

fn append_random(buf: &mut OsString, count: usize) {
    buf.reserve(count);
    let mut char_buf = [0u8; 4];
    for c in std::iter::repeat_with(fastrand::alphanumeric).take(count) {
        buf.push(c.encode_utf8(&mut char_buf));
    }
}

async fn generate_deletion_path(path: &Path) -> Result<PathBuf, AtomicReplaceError> {
    // tbd => to be deleted
    const PREFIX: &str = ".tbd-";
    const SUFFIX: &str = "-";
    const RAND_COUNT: usize = 6;
    let mut buf =
        OsString::with_capacity(path.as_os_str().len() + PREFIX.len() + RAND_COUNT + SUFFIX.len());
    buf.push(
        path.parent()
            .ok_or_else(|| AtomicReplaceError::InvalidTargetPath("Path must have a parent"))?
            .as_os_str(),
    );
    buf.push(std::path::MAIN_SEPARATOR_STR);
    buf.push(PREFIX);
    let trunc_len = buf.len();
    loop {
        append_random(&mut buf, RAND_COUNT);
        buf.push(SUFFIX);
        buf.push(
            path.file_name().ok_or_else(|| {
                AtomicReplaceError::InvalidTargetPath("Path must have a file name")
            })?,
        );
        if !tokio::fs::try_exists(Path::new(&buf))
            .await
            .map_err(AtomicReplaceError::PreModification)?
        {
            return Ok(PathBuf::from(buf));
        }
        buf.truncate(trunc_len);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AtomicReplaceError {
    #[error("Invalid target path: {0}")]
    InvalidTargetPath(&'static str),
    #[error("Failed pre-modification: {0}")]
    PreModification(#[source] std::io::Error),
    #[error("Failed to stage the original for deletion at {deletion_path:?}: {cause}")]
    StageForDeletion {
        deletion_path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
    #[error("{}", AtomicReplaceMoveReplacementDisplay { deletion_path: deletion_path, cause: cause })]
    MoveReplacement {
        deletion_path: Option<PathBuf>,
        #[source]
        cause: std::io::Error,
    },
    #[error("Failed to delete the original: {cause}. Remnants may be found at {deletion_path:?}.")]
    CleanUp {
        deletion_path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
}

struct AtomicReplaceMoveReplacementDisplay<'a> {
    deletion_path: &'a Option<PathBuf>,
    cause: &'a std::io::Error,
}

impl<'a> std::fmt::Display for AtomicReplaceMoveReplacementDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to move the replacement into place: {}.",
            self.cause
        )?;
        if let Some(deletion_path) = self.deletion_path {
            write!(f, " The original may be found at {deletion_path:?}.")
        } else {
            Ok(())
        }
    }
}

/// "Atomically" replaces `target` with `from`, which must be on the same file
/// system. If the operation fails, the original directory at `target`, if any,
/// will be left behind at a hidden path in the same parent directory as
/// `target`.
async fn atomic_replace(target: &Path, from: &Path) -> Result<(), AtomicReplaceError> {
    let metadata = match tokio::fs::metadata(target).await {
        Ok(t) => Some(t),
        Err(e) if e.is_not_found() => None,
        Err(e) => return Err(AtomicReplaceError::PreModification(e)),
    };
    let is_dir = metadata.map(|m| m.is_dir());
    let deletion_path = if is_dir.is_some() {
        let deletion_path = generate_deletion_path(target).await?;
        // Move the original to a hidden file just in case replacing it fails.
        if let Err(cause) = tokio::fs::rename(target, &deletion_path).await {
            return Err(AtomicReplaceError::StageForDeletion {
                deletion_path,
                cause,
            });
        }
        Some(deletion_path)
    } else {
        None
    };
    // If this fails, we will likely fail to restore the original, so don't
    // bother trying. Just let the user know where to find it.
    if let Err(cause) = tokio::fs::rename(from, target).await {
        return Err(AtomicReplaceError::MoveReplacement {
            deletion_path,
            cause,
        });
    }
    if let Some(deletion_path) = deletion_path {
        let is_dir = is_dir.unwrap();
        // The replacement has succeeded. Delete the original.
        if let Err(cause) = if is_dir {
            tokio::fs::remove_dir_all(&deletion_path).await
        } else {
            tokio::fs::remove_file(&deletion_path).await
        } {
            return Err(AtomicReplaceError::CleanUp {
                deletion_path,
                cause,
            });
        }
    }
    Ok(())
}

#[must_use]
pub struct StagedPackage<'a> {
    target: &'a Path,
    temp_dir: TempDir,
}

impl StagedPackage<'_> {
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Finishes installing the package by moving the staging directory into place,
    pub async fn finish(self, log: &slog::Logger) -> anyhow::Result<()> {
        atomic_replace(self.target, self.temp_dir.path()).await?;
        // the temp directory doesn't exist anymore.
        // without this, TempDir::drop would try to delete it
        _ = self.temp_dir.into_path();
        debug!(log, "Installed package to {:?}", self.target);
        Ok(())
    }
}

pub enum CacheKey<'a> {
    Hash(&'a str),
    Url,
}

pub struct CacheOptions<'a> {
    key: CacheKey<'a>,
    suffix: &'a str,
}

impl<'a> CacheOptions<'a> {
    pub fn by_hash(hash: &'a str) -> Self {
        Self {
            key: CacheKey::Hash(hash),
            suffix: "",
        }
    }

    pub fn by_url() -> Self {
        Self {
            key: CacheKey::Url,
            suffix: "",
        }
    }

    pub fn with_suffix(mut self, suffix: &'a str) -> Self {
        self.suffix = suffix;
        self
    }
}

pub enum FetchedResource {
    File(PathBuf),
    Bytes(BytesMut),
}

pub async fn fetch_resource<'a>(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    url: &str,
    cache: Option<CacheOptions<'_>>,
    task_id: Option<tasks::Id>,
) -> Result<FetchedResource> {
    match cache {
        Some(CacheOptions {
            key: CacheKey::Hash(hash_str),
            suffix,
        }) => fetch_resource_cached_by_hash(app, log, reqwest, url, hash_str, suffix, task_id)
            .await
            .map(FetchedResource::File),
        Some(CacheOptions {
            key: CacheKey::Url,
            suffix,
        }) => fetch_resource_cached_by_url(app, log, reqwest, url, suffix, task_id)
            .await
            .map(FetchedResource::File),
        None => fetch_resource_uncached(app, log, reqwest, url, task_id)
            .await
            .map(FetchedResource::Bytes),
    }
}

pub async fn fetch_resource_uncached<'a>(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    url: &str,
    task_id: Option<tasks::Id>,
) -> Result<BytesMut> {
    TaskBuilder::with_id(task_id.unwrap_or_else(tasks::allocate_task), url.to_owned())
        .kind(tasks::Kind::Download)
        .progress_unit(tasks::ProgressUnit::Bytes)
        .run_with_handle(app, |handle| async move {
            debug!(log, "Fetching resource from {url:?} without caching");

            let mut resp = reqwest.get(url).send().await?.error_for_status()?;
            let len = resp.content_length();
            let bytes = if let Some(len) = len {
                let len = usize::try_from(len).context("Too large to fit in memory")?;
                let mut bytes = BytesMut::with_capacity(len);
                let mut total = 0;
                while let Some(chunk) = resp.chunk().await? {
                    bytes.extend_from_slice(&chunk);
                    if let Some(app) = app {
                        total += chunk.len();
                        handle.send_progress_manually(app, total.as_u64(), len.as_u64())?;
                    }
                }
                bytes
            } else {
                let mut buf = Vec::new();
                let mut total = 0;
                while let Some(chunk) = resp.chunk().await? {
                    if let Some(app) = app {
                        total += chunk.len();
                        handle.send_progress_manually(app, total.as_u64(), 0)?;
                    }
                    buf.push(chunk);
                }
                let mut bytes = BytesMut::with_capacity(total);
                for chunk in buf {
                    bytes.extend_from_slice(&chunk);
                }
                bytes
            };

            Ok::<_, anyhow::Error>(bytes)
        })
        .await
        .map_err(Into::into)
}

pub async fn fetch_resource_cached_by_hash(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    url: &str,
    hash_str: &str,
    suffix: &str,
    task_id: Option<tasks::Id>,
) -> Result<PathBuf> {
    let mut path = cache_dir().join(hash_str);
    path.as_mut_os_string().push(suffix);

    fetch_resource_cached_by_hash_at_path(app, log, reqwest, url, hash_str, &path, task_id).await?;
    Ok(path)
}

pub async fn fetch_resource_cached_by_hash_at_path(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    url: &str,
    hash_str: &str,
    path: &Path,
    task_id: Option<tasks::Id>,
) -> Result<()> {
    TaskBuilder::with_id(task_id.unwrap_or_else(tasks::allocate_task), url.to_owned())
        .kind(tasks::Kind::Download)
        .progress_unit(tasks::ProgressUnit::Bytes)
        .run_with_handle(app, |handle| async move {
            debug!(log, "Fetching resource from {url:?} cached by hash");

            let hash = blake3::Hash::from_hex(hash_str)?;
            let hash_on_disk = {
                let mut hsr = blake3::Hasher::new();
                match hsr.update_mmap(&path) {
                    Ok(_) => Some(hsr.finalize()),
                    Err(e) if e.is_not_found() => None,
                    Err(e) => return Err(e.into()),
                }
            };
            if hash_on_disk.map(|h| h != hash).unwrap_or(true) {
                let mut resp = reqwest.get(url).send().await?.error_for_status()?;
                tokio::fs::create_dir_all(cache_dir()).await?;
                // TODO: should this be buffered?
                let mut wtr = tokio::fs::File::create(&path).await?;
                let mut written = 0u64;
                let len = resp.content_length();
                if let (Some(app), Some(total)) = (app, len) {
                    handle.send_progress_manually(app, written, total)?;
                }
                while let Some(chunk) = resp.chunk().await? {
                    wtr.write_all(&chunk).await?;
                    if let Some(app) = app {
                        written += chunk.len().as_u64();
                        handle.send_progress_manually(app, written, len.unwrap_or(0))?;
                    }
                }
                let hash_on_disk = {
                    let mut hsr = blake3::Hasher::new();
                    hsr.update_mmap(&path)?;
                    hsr.finalize()
                };
                debug!(log, "Cached resource at {path:?}");
                if hash_on_disk != hash {
                    bail!("Bad hash of downloaded resource at {path:?}: expected {hash}, found {hash_on_disk}");
                }
            } else {
                debug!(log, "Resource is cached at {path:?}");
                let metadata = tokio::fs::metadata(&path).await?;
                report_progress_from_file_metadata(app, handle, metadata)?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await
        .map_err(Into::into)
}

fn report_progress_from_file_metadata(
    app: Option<&AppHandle>,
    handle: TaskHandle,
    metadata: std::fs::Metadata,
) -> Result<(), anyhow::Error> {
    #[cfg(windows)]
    let size = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_size()
    };
    #[cfg(unix)]
    let size = {
        use std::os::unix::fs::MetadataExt;
        metadata.size()
    };
    if let Some(app) = app {
        handle.send_progress_manually(app, size, size)?;
    }
    Ok(())
}

pub async fn fetch_resource_cached_by_url(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    url: &str,
    suffix: &str,
    task_id: Option<tasks::Id>,
) -> Result<PathBuf> {
    TaskBuilder::with_id(task_id.unwrap_or_else(tasks::allocate_task), url.to_owned())
        .kind(tasks::Kind::Download)
        .progress_unit(tasks::ProgressUnit::Bytes)
        .run_with_handle(app, |handle| async move {
            debug!(log, "Fetching resource from {url:?} cached by url");

            let mut path = cache_dir().join("url.");
            path.as_mut_os_string()
                .push(base64::engine::general_purpose::URL_SAFE.encode(url));
            path.as_mut_os_string().push(suffix);
            match tokio::fs::metadata(&path).await {
                Ok(metadata) => {
                    debug!(log, "Resource is cached at {path:?}");
                    report_progress_from_file_metadata(app, handle, metadata)?;
                }
                Err(e) if e.is_not_found() => {
                    tokio::fs::create_dir_all(cache_dir()).await?;

                    let (tmp_file, tmp_path) = tokio::task::block_in_place(|| {
                        tempfile::NamedTempFile::new_in(
                            path.parent().context("path must have a parent")?,
                        )
                        .map_err(anyhow::Error::from)
                    })?
                    .into_parts();

                    let mut resp = reqwest.get(url).send().await?.error_for_status()?;

                    let tmp_file = tokio::fs::File::from_std(tmp_file);

                    let len = resp.content_length();
                    if let Some(len) = len {
                        tmp_file.allocate(len).await?;
                    }

                    // TODO: should this be buffered?
                    let mut wtr = tmp_file;
                    let mut written = 0u64;
                    if let (Some(app), Some(total)) = (app, len) {
                        handle.send_progress_manually(app, written, total)?;
                    }
                    while let Some(chunk) = resp.chunk().await? {
                        wtr.write_all(&chunk).await?;
                        if let Some(app) = app {
                            written += chunk.len().as_u64();
                            handle.send_progress_manually(app, written, len.unwrap_or(0))?;
                        }
                    }

                    let tmp_path = tmp_path.keep()?;
                    tokio::fs::rename(&tmp_path, &path)
                        .await
                        .context("Failed to move temp file into place")?;

                    debug!(log, "Cached resource at {path:?}");
                }
                Err(e) => return Err(e.into()),
            }
            Ok::<_, anyhow::Error>(path)
        })
        .await
        .map_err(Into::into)
}

pub async fn fetch_resource_as_bytes<'a>(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    url: &str,
    cache: Option<CacheOptions<'_>>,
    task_id: Option<tasks::Id>,
) -> Result<BytesMut> {
    match fetch_resource(app, log, reqwest, url, cache, task_id).await? {
        FetchedResource::File(path_buf) => {
            Ok(Bytes::from(tokio::fs::read(&path_buf).await?).into())
        }
        FetchedResource::Bytes(bytes) => Ok(bytes),
    }
}

/// Downloads a zip file from `url` and installs it into the `target` directory.
pub async fn install_zip<'a>(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    url: &str,
    cache: Option<CacheOptions<'_>>,
    target: &'a Path,
    task_id: Option<tasks::Id>,
) -> anyhow::Result<StagedPackage<'a>> {
    debug!(log, "Installing zip from {url:?} to {target:?}");

    let cache = cache.map(|c| c.with_suffix(".zip"));

    tokio::fs::create_dir_all(target)
        .await
        .context("Failed to create target directory")?;

    let target_parent = target
        .parent()
        .context("Target must not be a filesystem root")?;

    let mut changes = Vec::new();
    let changes = match scan_installed_package_for_changes(log, target, &mut changes).await {
        Ok(()) => Some(changes),
        Err(ScanError::IndexNotFoundError) => None,
        Err(e) => return Err(e.into()),
    };
    if let Some(changes) = &changes {
        debug!(log, "Zip is already installed to {target:?}");

        trace!(log, "Changes: {changes:#?}");
    }

    let temp_dir: TempDir;
    match fetch_resource(app, log, reqwest, url, cache, task_id).await? {
        FetchedResource::Bytes(bytes) => {
            temp_dir = tempfile::tempdir_in(target_parent)?;
            tokio::task::block_in_place(|| {
                let mut archive = ZipArchive::new(std::io::Cursor::new(bytes))?;
                archive.extract(temp_dir.path())?;
                Ok::<_, ZipError>(())
            })?;
        }
        FetchedResource::File(path) => {
            temp_dir = tempfile::tempdir_in(target_parent)?;
            tokio::task::block_in_place(|| {
                let mut archive =
                    ZipArchive::new(std::io::BufReader::new(std::fs::File::open(&path)?))?;
                archive.extract(temp_dir.path())?;
                Ok::<_, ZipError>(())
            })?;
        }
    }

    generate_package_index(log, temp_dir.path()).await?;

    if let Some(changes) = changes {
        let mut buf = temp_dir.path().to_owned();
        for (path, status) in changes {
            let rel_path = path.strip_prefix(target)?;
            buf.push(rel_path);
            debug!(log, "Preserving {rel_path:?} {status:?} across update");
            if matches!(status, Status::Deleted) {
                let metadata = tokio::fs::symlink_metadata(&buf).await?;
                match if metadata.is_dir() {
                    tokio::fs::remove_dir_all(&buf).await
                } else {
                    tokio::fs::remove_file(&buf).await
                } {
                    Ok(()) => {}
                    Err(e) if e.is_not_found() => {}
                    Err(e) => return Err(e.into()),
                }
            } else {
                merge_paths(log, &path, &buf).await?;
            }
            for _ in rel_path.components() {
                buf.pop();
            }
        }
    }

    Ok(StagedPackage { target, temp_dir })
}

/// Downloads a file from `url` and installs it at the `target` path.
pub async fn install_file<'a>(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    url: &str,
    cache: Option<CacheOptions<'_>>,
    target: &'a Path,
    task_id: Option<tasks::Id>,
) -> anyhow::Result<()> {
    debug!(log, "Installing file from {url:?} to {target:?}");

    let target_parent = target
        .parent()
        .context("Target must not be a filesystem root")?;

    tokio::fs::create_dir_all(target_parent)
        .await
        .context("Failed to create target parent directory")?;

    let mut temp_file = tempfile::NamedTempFile::new_in(target_parent)?;
    let temp_path;
    match fetch_resource(app, log, reqwest, url, cache, task_id).await? {
        FetchedResource::Bytes(bytes) => {
            tokio::task::block_in_place(|| temp_file.write_all(&bytes))?;
            temp_path = temp_file.into_temp_path();
        }
        FetchedResource::File(path) => {
            temp_path = temp_file.into_temp_path();
            tokio::fs::copy(&path, &temp_path).await?;
        }
    }

    tokio::task::block_in_place(|| temp_path.persist(target))?;

    Ok(())
}

pub async fn uninstall_package<'a>(
    log: &slog::Logger,
    path: &'a Path,
    keep_changes: bool,
) -> anyhow::Result<()> {
    if keep_changes {
        let mut changes = TrieBuilder::new();
        struct ExtendByFn<F>(F);
        impl<F, I> Extend<I> for ExtendByFn<F>
        where
            F: FnMut(I),
        {
            fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
                iter.into_iter().for_each(&mut self.0);
            }
        }
        scan_installed_package_for_changes(
            log,
            path,
            &mut ExtendByFn(|(path, status): (PathBuf, _)| {
                if !matches!(status, Status::Deleted) {
                    changes.insert(path.components().map(|c| c.as_os_str().to_owned()));
                }
            }),
        )
        .await?;
        let changes = changes.build();

        debug!(log, "Changes: {changes:?}");

        let mut iter = WalkDir::new(path).into_iter();
        ensure!(
            iter.next().context("Expected root entry")??.path() == path,
            "First entry was not root"
        );
        while let Some(r) = iter.next() {
            let e = r?;
            #[derive(Clone, Copy)]
            struct Discard;
            impl<A> FromIterator<A> for Discard {
                fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
                    iter.into_iter().for_each(|_| {});
                    Self
                }
            }
            // TODO: avoid cloning and collecting
            if changes
                .predictive_search::<Discard, _>(
                    e.path()
                        .components()
                        .map(|c| c.as_os_str().to_owned())
                        .collect::<Vec<_>>(),
                )
                .next()
                .is_none()
            {
                if e.file_type().is_dir() {
                    debug!(log, "Removing directory tree at {:?}", e.path());
                    tokio::fs::remove_dir_all(e.path()).await?;
                    iter.skip_current_dir();
                } else {
                    debug!(log, "Removing file at {:?}", e.path());
                    tokio::fs::remove_file(e.path()).await?;
                }
            }
        }
    } else {
        tokio::fs::remove_dir_all(path).await?;
    }

    debug!(log, "Uninstalled package from {path:?}");

    Ok(())
}

async fn merge_paths(log: &slog::Logger, from: &Path, to: &Path) -> Result<()> {
    let mut iter = WalkDir::new(from).into_iter();
    while let Some(r) = iter.next() {
        let dir_entry = r?;
        let rel_path = dir_entry.path().strip_prefix(from).context("unreachable")?;
        let to = if rel_path == Path::new("") {
            to.to_owned()
        } else {
            to.join(rel_path)
        };
        trace!(
            log,
            "Merging {:?} ({rel_path:?}) into {to:?}",
            dir_entry.path()
        );
        async {
            enum FileType {
                FileLike,
                Dir,
            }
            let file_type = match tokio::fs::symlink_metadata(&to).await {
                Ok(metadata) if metadata.is_dir() => Some(FileType::Dir),
                Ok(_) => Some(FileType::FileLike),
                Err(e) if e.is_not_found() => None,
                Err(e) => return Err(anyhow::Error::from(e)),
            };
            match (dir_entry.file_type().is_dir(), file_type) {
                (true, Some(FileType::Dir)) => {
                    // both are directories, so we want to overlay
                    return Ok(());
                }
                (true, Some(FileType::FileLike)) => {
                    // target is not a directory, so skip walking and we'll just rename
                    // iter.skip_current_dir();
                    // remove the target file and we'll just rename
                    tokio::fs::remove_file(&to).await?;
                }
                (true, None) => {
                    // target is not a directory, so skip walking and we'll just rename
                    // iter.skip_current_dir();
                }
                (false, Some(FileType::Dir)) => {
                    // source is not a directory, so remove the target directory and we'll just rename
                    tokio::fs::remove_dir_all(&to).await?;
                }
                (false, Some(FileType::FileLike) | None) => {}
            }
            // `rename` could be faster, but would leave the installation
            // corrupted if the merge fails. Instead, just copy files all the
            // way down.
            if dir_entry.file_type().is_dir() {
                tokio::fs::create_dir(&to).await?;
            } else {
                tokio::fs::copy(dir_entry.path(), &to).await?;
            }
            Result::Ok(())
        }
        .await
        .with_context(|| format!("Failed to merge {:?} into {:?}", dir_entry.path(), to))?;
    }
    Ok(())
}

pub async fn clear_cache() -> Result<()> {
    let cache_dir = cache_dir();
    match tokio::fs::remove_dir_all(&cache_dir).await {
        Ok(()) => {}
        Err(e) if e.is_not_found() => {
            if tokio::fs::try_exists(&cache_dir).await? {
                // failed on a sub-path
                return Err(e.into());
            }
            // the thing we wanted to delete didn't exist, this is good!
        }
        Err(e) => return Err(e.into()),
    }
    tokio::fs::create_dir(&cache_dir).await?;
    Ok(())
}
