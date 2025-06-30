//! Importing profiles that have been shared on Thunderstore.

use std::{
    borrow::Cow,
    io::Read,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::{bail, ensure, Context, Result};
use base64::prelude::BASE64_STANDARD;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use triomphe::Arc;
use uuid::Uuid;
use zip::read::ZipFile;

use crate::{profiles::{CONFIG_FOLDER, PATCHERS_FOLDER}, Reqwest};
use crate::{installing::fetch_resource_as_bytes, profiles::MODS_FOLDER, tasks};

#[derive(Clone)]
pub struct FullName {
    value: String,
    split: usize,
}

impl FullName {
    pub fn namespace(&self) -> &str {
        &self.value[..self.split]
    }

    pub fn name(&self) -> &str {
        &self.value[self.split + 1..]
    }

    pub fn components(&self) -> (&str, &str) {
        (self.namespace(), self.name())
    }
}

impl Deref for FullName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl From<FullName> for String {
    fn from(value: FullName) -> Self {
        value.value
    }
}

impl std::fmt::Display for FullName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}

impl std::fmt::Debug for FullName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullName")
            .field("namespace", &self.namespace())
            .field("name", &self.name())
            .finish()
    }
}

impl<'de> serde::Deserialize<'de> for FullName {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = FullName;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string containing a hyphen separated namespace and name")
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let split = v.find('-').ok_or_else(|| {
                    E::invalid_value(
                        serde::de::Unexpected::Str(v),
                        &"a hyphen separated namespace and name",
                    )
                })?;
                Ok(FullName {
                    value: v.to_owned(),
                    split,
                })
            }

            fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let split = v.find('-').ok_or_else(|| {
                    E::invalid_value(
                        serde::de::Unexpected::Str(&v),
                        &"a hyphen separated namespace and name",
                    )
                })?;
                Ok(FullName { value: v, split })
            }
        }

        deserializer.deserialize_string(Visitor)
    }
}

impl serde::Serialize for FullName {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

// TODO: replace with a custom deserializer instead of needing two layers of validation
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl TryFrom<Version> for packed_semver::Version {
    type Error = packed_semver::TooManyBitsError;

    fn try_from(value: Version) -> Result<Self, Self::Error> {
        Self::new(value.major, value.minor, value.patch)
    }
}

#[derive(Debug)]
pub struct Profile {
    pub manifest: ProfileManifest,
    pub archive: zip::ZipArchive<std::io::Cursor<Arc<[u8]>>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileManifest {
    pub profile_name: String,
    pub mods: Vec<ProfileMod>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProfileMod {
    #[serde(rename = "name")]
    pub full_name: FullName,
    #[serde(alias = "versionNumber")]
    pub version: Version,
    pub enabled: bool,
}

const R2_PROFILE_DATA_PREFIX: &str = "#r2modman\n";

pub const R2_PROFILE_MANIFEST_FILE_NAME: &str = "export.r2x";

pub async fn lookup_profile(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    reqwest: &Reqwest,
    id: Uuid,
    task_id: Option<tasks::Id>,
) -> Result<Profile> {
    let bytes = fetch_resource_as_bytes(
        app,
        log,
        reqwest,
        &format!("https://thunderstore.io/api/experimental/legacyprofile/get/{id}/"),
        Some(crate::installing::CacheOptions::by_url().with_suffix(".r2z")),
        task_id,
    )
    .await?;

    tokio::task::block_in_place(move || {
        let Some((prefix, bytes)) = bytes.split_at_checked(R2_PROFILE_DATA_PREFIX.len()) else {
            bail!("Invalid profile data")
        };
        ensure!(
            prefix == R2_PROFILE_DATA_PREFIX.as_bytes(),
            "Invalid profile data"
        );

        let mut buf = Vec::new();
        base64::read::DecoderReader::new(std::io::Cursor::new(bytes), &BASE64_STANDARD)
            .read_to_end(&mut buf)
            .context("Failed to decode base64 data")?;

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(Arc::from(buf)))?;

        let manifest_file = archive
            .by_name("export.r2x")
            .context("Profile archive is missing manifest file")?;

        let manifest = serde_yaml::from_reader(manifest_file)?;

        Ok(Profile { manifest, archive })
    })
}

pub fn get_archive_file_path<R: Read>(file: &ZipFile<'_, R>) -> Result<Option<PathBuf>> {
    let path = file
        .enclosed_name()
        .with_context(|| format!("File in archive has a bad path: {:?}", file.name()))?;
    if path.as_os_str() == R2_PROFILE_MANIFEST_FILE_NAME {
        return Ok(None);
    }
    let path = if let Ok(p) = path.strip_prefix("BepInEx") {
        Cow::Borrowed(p)
    } else {
        Cow::Owned(path)
    };
    let path = {
        let mut iter = path.components();
        match iter.next() {
            Some(first) if first.as_os_str() == "plugins" => {
                Cow::Owned(Path::new(MODS_FOLDER).join(iter.as_path()))
            }
            _ => path,
        }
    };

    match path.components().next().map(|s| s.as_os_str()) {
        Some(s) if s == MODS_FOLDER || s == CONFIG_FOLDER || s == PATCHERS_FOLDER => {}
        _ => return Ok(None),
    }

    Ok(Some(path.into_owned()))
}
