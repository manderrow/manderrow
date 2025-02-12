//! Importing profiles that have been shared on Thunderstore.

use std::io::Read;

use anyhow::{ensure, Context, Result};
use base64::prelude::BASE64_STANDARD;
use serde::{Deserialize, Serialize};
use tauri::utils::acl::manifest::Manifest;
use uuid::Uuid;

use crate::{http::fetch_as_blocking, Reqwest};

#[derive(Debug)]
pub struct Profile {
    pub manifest: Manifest,
    pub archive: zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileManifest {
    pub profile_name: String,
    pub mods: Vec<Mod>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Mod {
    #[serde(rename = "name")]
    pub full_name: String,
    #[serde(alias = "versionNumber")]
    pub version: Version,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

const PROFILE_DATA_PREFIX: &str = "#r2modman\n";

async fn lookup_profile(client: &Reqwest, id: Uuid) -> Result<Profile> {
    let mut rdr = fetch_as_blocking(client.get(format!(
        "https://thunderstore.io/api/experimental/legacyprofile/get/{id}/"
    )))
    .await?;

    tokio::task::block_in_place(move || {
        {
            const BUF_LEN: usize = PROFILE_DATA_PREFIX.len();
            let mut buf = [0u8; BUF_LEN];
            rdr.read_exact(&mut buf)?;
            ensure!(
                buf == PROFILE_DATA_PREFIX.as_bytes(),
                "Invalid profile data"
            );
        }

        let mut buf = Vec::new();
        base64::read::DecoderReader::new(rdr, &BASE64_STANDARD)
            .read_to_end(&mut buf)
            .context("Failed to decode base64 data")?;

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(buf))?;

        let manifest_file = archive
            .by_name("export.r2x")
            .context("Profile archive is missing manifest file")?;

        let manifest = serde_yaml::from_reader(manifest_file)?;

        Ok(Profile { manifest, archive })
    })
}
