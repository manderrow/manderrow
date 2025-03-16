use std::{borrow::Cow, collections::HashMap, ffi::OsStr, hash::Hash, path::Path};

use itertools::Itertools;
use rkyv::vec::ArchivedVec;

#[derive(Debug, Clone, PartialEq, Eq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(derive(Debug, PartialEq, Eq))]
#[rkyv(compare(PartialEq))]
pub enum NativePath {
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

pub struct ArchivedNativePathComponents<'a> {
    #[cfg(unix)]
    iter: std::slice::Iter<'a, ArchivedVec<u8>>,
    #[cfg(windows)]
    iter: std::slice::Iter<'a, ArchivedVec<rkyv::rend::u16_le>>,
}

impl<'a> Iterator for ArchivedNativePathComponents<'a> {
    type Item = Cow<'a, OsStr>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            self.iter
                .next()
                .map(|component| OsStr::from_bytes(component).into())
        }
        #[cfg(windows)]
        {
            // TODO: if host is little-endian, cast instead of mapping and collecting into an intermediate buffer
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;
            self.iter.next().map(|component| {
                OsString::from_wide(&component.iter().map(|c| c.to_native()).collect::<Vec<_>>())
                    .into()
            })
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
pub struct PathAsNativePath<'a>(&'a Path);

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
                                    .all(|item| {
                                        item.both().map(|(a, b)| a == *b).unwrap_or_default()
                                    })
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
pub enum Index {
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
pub enum IndexEntryRef<'a> {
    V1(&'a ArchivedIndexEntryV1),
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(derive(Debug))]
pub enum IndexEntryV1 {
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
pub struct IndexPath(pub Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Path was not valid Unicode")]
pub struct IndexPathFromPathError;

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
