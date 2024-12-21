//! Package installation helpers
//!
//! Never make changes to `IndexEntryV*` or [`Index`] variants. Make a new version instead.

use std::{
    borrow::Cow, collections::{HashMap, HashSet}, ffi::OsStr, hash::Hash, path::{Path, PathBuf}
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use itertools::Itertools;
use log::{debug, trace};
use rkyv::{rend::u16_le, vec::ArchivedVec};
use tauri_plugin_http::reqwest;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use trie_rs::TrieBuilder;
use walkdir::WalkDir;
use zip::{result::ZipError, ZipArchive};

use crate::{paths::cache_dir, util::IoErrorKindExt};

#[derive(Debug, Clone, PartialEq, Eq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(derive(Debug, PartialEq, Eq))]
#[rkyv(compare(PartialEq))]
enum NativePath {
    Unix(Vec<Vec<u8>>),
    Windows(Vec<Vec<u16>>),
}

impl NativePath {
    pub fn component_count(&self) -> usize {
        match self {
            Self::Unix(vec) => vec.len(),
            Self::Windows(vec) => vec.len(),
        }
    }
}

impl ArchivedNativePath {
    pub fn component_count(&self) -> usize {
        match self {
            Self::Unix(vec) => vec.len(),
            Self::Windows(vec) => vec.len(),
        }
    }

    pub fn components(&self) -> ArchivedNativePathComponents {
        match self {
            #[cfg(unix)]
            ArchivedNativePath::Unix(vec) => ArchivedNativePathComponents { iter: vec.iter() },
            #[cfg(windows)]
            ArchivedNativePath::Windows(vec) => ArchivedNativePathComponents { iter: vec.iter() },
            _ => panic!("Attempted to use an index across operating systems"),
        }
    }
}

struct ArchivedNativePathComponents<'a> {
    #[cfg(unix)]
    iter: std::slice::Iter<'a, ArchivedVec<u8>>,
    #[cfg(windows)]
    windows: std::slice::Iter<'a, ArchivedVec<u16_le>>,
}

impl<'a> Iterator for ArchivedNativePathComponents<'a> {
    type Item = Cow<'a, OsStr>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            self.iter.next().map(|component| OsStr::from_bytes(component).into())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            self.iter.next().map(|component| OsString::from_wide(component).into())
        }
    }
}

impl<T: AsRef<Path>> From<T> for NativePath {
    fn from(value: T) -> Self {
        let value = value.as_ref();
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            Self::Unix(
                value
                    .components()
                    .map(|s| s.as_os_str().as_bytes().to_owned())
                    .collect(),
            )
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            Self::Windows(
                value
                    .components()
                    .map(|s| s.as_os_str().encode_wide().collect::<Vec<_>>())
                    .collect(),
            )
        }
    }
}

impl Hash for NativePath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            #[cfg(unix)]
            Self::Unix(vec) => vec.hash(state),
            #[cfg(windows)]
            Self::Windows(vec) => vec.hash(state),
            _ => panic!("Attempted to use an index across operating systems"),
        }
    }
}

impl Hash for ArchivedNativePath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            #[cfg(unix)]
            Self::Unix(vec) => vec.hash(state),
            #[cfg(windows)]
            Self::Windows(vec) => vec.hash(state),
            _ => panic!("Attempted to use an index across operating systems"),
        }
    }
}

#[derive(PartialEq, Eq)]
struct PathAsNativePath<'a>(&'a Path);

impl<'a> Hash for PathAsNativePath<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            self.0.components().count().hash(state);
            self.0
                .components()
                .map(|s| s.as_os_str().as_bytes())
                .for_each(|component| component.hash(state));
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            self.0.components().count().hash(state);
            self.0.components().for_each(|s| {
                s.as_os_str().encode_wide().count().hash(state);
                s.as_os_str()
                    .encode_wide()
                    .for_each(|element| element.hash(state));
            });
        }
    }
}

impl<'a> PartialEq<ArchivedNativePath> for Path {
    fn eq(&self, other: &ArchivedNativePath) -> bool {
        match other {
            #[cfg(unix)]
            ArchivedNativePath::Unix(components) => {
                use std::os::unix::ffi::OsStrExt;
                self.components()
                    .zip_longest(components.iter())
                    .all(|item| {
                        item.both()
                            .map(|(a, b)| a.as_os_str().as_bytes() == b)
                            .unwrap_or_default()
                    })
            }
            #[cfg(windows)]
            ArchivedNativePath::Windows(components) => {
                use std::os::windows::ffi::OsStrExt;
                self.components()
                    .zip_longest(components.iter())
                    .all(|item| {
                        item.both()
                            .map(|(a, b)| {
                                a.as_os_str()
                                    .encode_wide()
                                    .zip_longest(b.iter())
                                    .map(|(a, b)| a == b)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default()
                    })
            }
            _ => panic!("Attempted to use an index across operating systems"),
        }
    }
}

/// Index of files that came with the zip.
#[derive(Debug, Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(derive(Debug))]
enum Index {
    V1(HashMap<IndexPath, IndexEntryV1>),
    V2(HashMap<NativePath, IndexEntryV1>),
}

impl ArchivedIndex {
    pub fn get<'a>(&'a self, path: &Path) -> Option<IndexEntryRef<'a>> {
        match self {
            ArchivedIndex::V1(entries) => entries
                .get_with(&IndexPath::try_from(path).ok()?, |a, b| a == b)
                .map(IndexEntryRef::V1),
            ArchivedIndex::V2(entries) => entries
                .get_with(&PathAsNativePath(path), |a, b| a.0 == b)
                .map(IndexEntryRef::V1),
        }
    }
}

#[derive(Debug, Clone)]
enum IndexEntryRef<'a> {
    V1(&'a ArchivedIndexEntryV1),
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(derive(Debug))]
enum IndexEntryV1 {
    File {
        hash: [u8; blake3::OUT_LEN],
    },
    Symlink {
        /// This will be relative if it points inside the package directory.
        target: String,
    },
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(derive(Debug, PartialEq, Eq, Hash))]
#[rkyv(compare(PartialEq))]
struct IndexPath(Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Path was not valid Unicode")]
struct IndexPathFromPathError;

impl<'a> TryFrom<&'a Path> for IndexPath {
    type Error = IndexPathFromPathError;

    fn try_from(value: &'a Path) -> Result<Self, Self::Error> {
        value
            .components()
            .map(|s| {
                s.as_os_str()
                    .to_owned()
                    .into_string()
                    .map_err(|_| IndexPathFromPathError)
            })
            .collect::<Result<_, _>>()
            .map(IndexPath)
    }
}

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
    path: &Path,
    buf: &mut impl Extend<(PathBuf, Status)>,
) -> Result<(), ScanError> {
    let mut index_buf = Vec::new();
    scan_installed_package_for_changes_with_index_buf(path, buf, &mut index_buf).await?;
    Ok(())
}

async fn scan_installed_package_for_changes_with_index_buf<'i>(
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

    // let mut seen = HashSet::new();

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
            // seen.insert(rel_path.to_owned());
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
                        trace!("Not recording deletion because a parent was also deleted: {e_path:?} is inside of {entry:?}");
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
                                b.as_os_str()
                                    .to_str()
                                    .map(|b| &*a == b)
                                    .unwrap_or(false)
                            })
                    }) {
                        trace!("Not recording deletion because a parent was also deleted: {indexed_path:?} is inside of {entry:?}");
                    } else {
                        buf.extend_one((p, Status::Deleted));
                    }
                }
            }
        }
        None => {}
    }

    trace!("Index: {index:#?}");

    Ok(index)
}

async fn generate_package_index(path: &Path) -> Result<()> {
    debug!("Generating package index for {path:?}");

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

#[must_use]
pub struct StagedPackage<'a> {
    target: &'a Path,
    temp_dir: TempDir,
}

impl StagedPackage<'_> {
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    pub async fn finish(self) -> anyhow::Result<()> {
        match tokio::fs::remove_dir_all(&self.target).await {
            Ok(()) => {}
            Err(e) if e.is_not_found() => {}
            Err(e) => return Err(e).context("Unable to remove previous installation"),
        }
        tokio::fs::rename(self.temp_dir.into_path(), &self.target)
            .await
            .context("Unable to move temporary directory into place")?;
        debug!("Installed package to {:?}", self.target);
        Ok(())
    }
}

/// Downloads a zip file from `url` and installs it into the `target`
/// directory. If `hash_str` is provided, it will be used to cache the zip file
/// for future reuse.
pub async fn install_zip<'a>(
    url: &str,
    hash_str: Option<&str>,
    target: &'a Path,
) -> anyhow::Result<StagedPackage<'a>> {
    debug!("Installing zip from {url:?} to {target:?}");

    let target_parent = target
        .parent()
        .context("Target must not be a filesystem root")?;

    let mut changes = Vec::new();
    let changes = match scan_installed_package_for_changes(target, &mut changes).await {
        Ok(()) => Some(changes),
        Err(ScanError::IndexNotFoundError) => None,
        Err(e) => return Err(e.into()),
    };
    if let Some(changes) = &changes {
        debug!("Zip is already installed to {target:?}");

        trace!("Changes: {changes:#?}");
    }

    let temp_dir: TempDir;
    if let Some(hash_str) = hash_str {
        let path = cache_dir().join(format!("{hash_str}.zip"));
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
            let mut resp = reqwest::get(url).await?.error_for_status()?;
            let mut wtr = tokio::fs::File::create(&path).await?;
            while let Some(chunk) = resp.chunk().await? {
                wtr.write_all(&chunk).await?;
            }
            debug!("Cached zip at {path:?}");
        } else {
            debug!("Zip is cached at {path:?}");
        }

        temp_dir = tempfile::tempdir_in(target_parent)?;
        tokio::task::block_in_place(|| {
            let mut archive =
                ZipArchive::new(std::io::BufReader::new(std::fs::File::open(&path)?))?;
            archive.extract(temp_dir.path())?;
            Ok::<_, ZipError>(())
        })?;
    } else {
        let resp = reqwest::get(url).await?.error_for_status()?;
        let bytes = resp.bytes().await?;

        temp_dir = tempfile::tempdir_in(target_parent)?;
        tokio::task::block_in_place(|| {
            let mut archive =
                ZipArchive::new(std::io::BufReader::new(std::io::Cursor::new(bytes)))?;
            archive.extract(temp_dir.path())?;
            Ok::<_, ZipError>(())
        })?;
    }

    generate_package_index(temp_dir.path()).await?;

    if let Some(changes) = changes {
        let mut buf = temp_dir.path().to_owned();
        for (path, status) in changes {
            let rel_path = path.strip_prefix(target)?;
            buf.push(rel_path);
            debug!("Preserving {rel_path:?} {status:?} across update");
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
                merge_paths(&path, &buf).await?;
            }
            for _ in rel_path.components() {
                buf.pop();
            }
        }
    }

    Ok(StagedPackage { target, temp_dir })
}

pub async fn uninstall_package<'a>(path: &'a Path, keep_changes: bool) -> anyhow::Result<()> {
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
            path,
            &mut ExtendByFn(|(path, status): (PathBuf, _)| {
                if !matches!(status, Status::Deleted) {
                    changes.insert(path.components().map(|c| c.as_os_str().to_owned()));
                }
            }),
        )
        .await?;
        let changes = changes.build();

        debug!("Changes: {changes:?}");

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
            if changes.predictive_search::<Discard, _>(e.path().components().map(|c| c.as_os_str().to_owned()).collect::<Vec<_>>()).next().is_none() {
                if e.file_type().is_dir() {
                    debug!("Removing directory tree at {:?}", e.path());
                    tokio::fs::remove_dir_all(e.path()).await?;
                    iter.skip_current_dir();
                } else {
                    debug!("Removing file at {:?}", e.path());
                    tokio::fs::remove_file(e.path()).await?;
                }
            }
        }
    } else {
        tokio::fs::remove_dir_all(path).await?;
    }

    debug!("Uninstalled package from {path:?}");

    Ok(())
}

async fn merge_paths(from: &Path, to: &Path) -> Result<()> {
    let mut iter = WalkDir::new(from).into_iter();
    while let Some(r) = iter.next() {
        let dir_entry = r?;
        let rel_path = dir_entry.path().strip_prefix(from).context("unreachable")?;
        let to = if rel_path == Path::new("") {
            to.to_owned()
        } else {
            to.join(rel_path)
        };
        trace!("Merging {:?} ({rel_path:?}) into {to:?}", dir_entry.path());
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
