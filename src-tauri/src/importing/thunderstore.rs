//! Importing profiles that have been shared on Thunderstore.

use std::{
    borrow::Cow,
    io::Read,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use base64::prelude::BASE64_STANDARD;
use saphyr::LoadableYamlNode;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use triomphe::Arc;
use uuid::Uuid;
use zip::read::ZipFile;

use crate::Reqwest;
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

#[derive(Debug, thiserror::Error)]
enum ParseFullNameError<'a> {
    #[error("expected a hyphen separated namespace and name: {0:?}")]
    MissingHyphen(&'a str),
}

impl FullName {
    fn from_str(s: &str) -> Result<Self, ParseFullNameError<'_>> {
        let split = s.find('-').ok_or(ParseFullNameError::MissingHyphen(s))?;
        Ok(FullName {
            value: s.to_owned(),
            split,
        })
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
                FullName::from_str(v).map_err(E::custom)
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
    pub version: packed_semver::Version,
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

        let mut manifest = String::new();
        archive
            .by_name("export.r2x")
            .context("Profile archive is missing manifest file")?
            .read_to_string(&mut manifest)
            .context("Failed to read manifest file")?;

        let manifest =
            saphyr::Yaml::load_from_str(&manifest).context("Failed to parse manifest file")?;
        if manifest.len() != 1 {
            bail!(
                "Unexpected YAML document count in manifest file: {}",
                manifest.len()
            );
        }

        let Some(manifest) = manifest[0].as_mapping() else {
            bail!(
                "Unexpected root YAML element in manifest file: {:?}",
                manifest[0]
            );
        };

        let Some(profile_name) = manifest.get(&saphyr::Yaml::Value(saphyr::Scalar::String(
            Cow::Borrowed("profileName"),
        ))) else {
            return Err(MissingPropertyError::new(&[], "profileName").into());
        };

        let Some(profile_name) = profile_name.as_str() else {
            bail!("Invalid value of property profileName in manifest file");
        };

        let Some(mods) = manifest.get(&saphyr::Yaml::Value(saphyr::Scalar::String(Cow::Borrowed(
            "mods",
        )))) else {
            return Err(MissingPropertyError::new(&[], "mods").into());
        };

        let Some(mods) = mods.as_sequence() else {
            bail!("Invalid value of property mods in manifest file");
        };

        let mods = mods
            .iter()
            .map(|m| {
                let Some(m) = m.as_mapping() else {
                    bail!("Unexpected root YAML element in manifest file: {m:?}");
                };

                let Some(full_name) = m.get(&saphyr::Yaml::Value(saphyr::Scalar::String(
                    Cow::Borrowed("name"),
                ))) else {
                    return Err(MissingPropertyError::new(&["mods"], "name").into());
                };
                let Some(full_name) = full_name.as_str() else {
                    return Err(InvalidPropertyValueError::new(&["mods"], "name", full_name).into());
                };

                let Some(version) = m.get(&saphyr::Yaml::Value(saphyr::Scalar::String(
                    Cow::Borrowed("versionNumber"),
                ))) else {
                    return Err(MissingPropertyError::new(&["mods"], "versionNumber").into());
                };
                let Some(version) = version.as_mapping() else {
                    return Err(InvalidPropertyValueError::new(
                        &["mods"],
                        "versionNumber",
                        version,
                    )
                    .into());
                };

                let major_version = parse_version_component(version, "major")?;
                let minor_version = parse_version_component(version, "minor")?;
                let patch_version = parse_version_component(version, "patch")?;

                let Some(enabled) = m.get(&saphyr::Yaml::Value(saphyr::Scalar::String(
                    Cow::Borrowed("enabled"),
                ))) else {
                    return Err(MissingPropertyError::new(&["mods"], "enabled").into());
                };
                let Some(enabled) = enabled.as_bool() else {
                    return Err(
                        InvalidPropertyValueError::new(&["mods"], "enabled", enabled).into(),
                    );
                };

                Ok(ProfileMod {
                    full_name: FullName::from_str(full_name).map_err(|e| anyhow!("{e}"))?,
                    version: packed_semver::Version::new(
                        major_version,
                        minor_version,
                        patch_version,
                    )?,
                    enabled,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Profile {
            manifest: ProfileManifest {
                profile_name: profile_name.to_owned(),
                mods,
            },
            archive,
        })
    })
}

#[derive(Debug)]
struct MissingPropertyError {
    parents: &'static [&'static str],
    key: &'static str,
}

impl MissingPropertyError {
    pub const fn new(parents: &'static [&'static str], key: &'static str) -> Self {
        Self { parents, key }
    }
}

fn write_property_error_common(
    f: &mut std::fmt::Formatter<'_>,
    prefix: &str,
    key: &str,
    parents: &[&str],
) -> std::fmt::Result {
    f.write_str(prefix)?;
    f.write_str(key)?;
    f.write_str(" in manifest file")?;
    for parent in parents {
        f.write_str(" -> ")?;
        f.write_str(parent)?;
    }
    Ok(())
}

impl std::fmt::Display for MissingPropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_property_error_common(f, "Missing property ", self.key, self.parents)
    }
}

impl std::error::Error for MissingPropertyError {}

#[derive(Debug)]
struct InvalidPropertyValueError {
    parents: &'static [&'static str],
    key: &'static str,
    value: String,
}

impl InvalidPropertyValueError {
    pub fn new(parents: &'static [&'static str], key: &'static str, value: &saphyr::Yaml) -> Self {
        Self {
            parents,
            key,
            value: format!("{value:?}"),
        }
    }
}

impl std::fmt::Display for InvalidPropertyValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_property_error_common(f, "Invalid value of property ", self.key, self.parents)?;
        f.write_str(": ")?;
        f.write_str(&self.value)?;
        Ok(())
    }
}

impl std::error::Error for InvalidPropertyValueError {}

fn parse_version_component(version: &saphyr::Mapping, key: &'static str) -> Result<u64> {
    let Some(comp) = version.get(&saphyr::Yaml::Value(saphyr::Scalar::String(Cow::Borrowed(
        key,
    )))) else {
        return Err(MissingPropertyError::new(&["mods", "version"], key).into());
    };
    let Some(comp) = comp.as_integer().and_then(|i| i.try_into().ok()) else {
        return Err(InvalidPropertyValueError::new(&["mods", "version"], key, comp).into());
    };
    Ok(comp)
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
        Some(s) if s == MODS_FOLDER || s == "config" || s == "patchers" => {}
        _ => return Ok(None),
    }

    Ok(Some(path.into_owned()))
}
