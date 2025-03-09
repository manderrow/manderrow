use std::collections::HashSet;

use anyhow::{anyhow, bail, Context};
use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use serde::Serialize;
use tauri::State;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use uuid::Uuid;

use crate::mods::{ModId, ModMetadata, ModVersion, Version};
use crate::profiles::profile_path;
use crate::{CommandError, Reqwest};

use super::thunderstore;

#[derive(Debug, Clone, Serialize)]
pub struct Modpack {
    pub name: String,
    pub mods: Vec<ModSpec>,
    pub diff: Vec<PathDiff>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ModSpec {
    /// A mod pulled from somewhere online.
    Online { url: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct PathDiff {
    pub path: String,
    pub diff: Diff,
}

#[derive(Debug, Clone, Serialize)]
pub enum Diff {
    Created,
    Deleted,
    Modified,
}

#[tauri::command]
pub async fn preview_import_modpack_from_thunderstore_code(
    reqwest: State<'_, Reqwest>,
    thunderstore_id: Uuid,
    game: &str,
    profile_id: Option<Uuid>,
) -> Result<Modpack, CommandError> {
    let mut profile = thunderstore::lookup_profile(&reqwest, thunderstore_id).await?;

    let mut mods = Vec::with_capacity(profile.manifest.mods.len());

    for m in profile.manifest.mods {
        let (namespace, name) = m.full_name.components();
        mods.push_within_capacity(ModSpec::Online {
            url: format!(
                "https://gcdn.thunderstore.io/live/repository/packages/{namespace}-{name}-{}.zip",
                m.version
            ),
        })
        .unwrap();
    }

    let mut diff = Vec::with_capacity(profile.archive.len());

    for i in 0..profile.archive.len() {
        let file = profile
            .archive
            .by_index(i)
            .context("Failed to open file in archive")?;

        let Some(path) = thunderstore::get_archive_file_path(&file)? else {
            continue;
        };

        let path = path
            .into_os_string()
            .into_string()
            .map_err(|s| anyhow!("Path must be valid Unicode: {s:?}"))?;

        diff.push_within_capacity(PathDiff {
            path,
            diff: Diff::Created,
        })
        .unwrap();
    }

    Ok(Modpack {
        name: profile.manifest.profile_name,
        mods,
        diff,
    })
}

// TODO: don't fetch again
#[tauri::command]
pub async fn import_modpack_from_thunderstore_code(
    reqwest: State<'_, Reqwest>,
    thunderstore_id: Uuid,
    game: &str,
    profile_id: Option<Uuid>,
) -> Result<Uuid, CommandError> {
    if profile_id.is_some() {
        return Err(anyhow!("Importing over existing profiles is not yet supported").into());
    }

    _ = profile_id;
    let profile = thunderstore::lookup_profile(&reqwest, thunderstore_id).await?;

    let profile_id = match profile_id {
        Some(profile_id) => profile_id,
        None => {
            crate::profiles::create_profile(game.into(), profile.manifest.profile_name.into())
                .await?
        }
    };

    profile
        .manifest
        .mods
        .iter()
        .map(|m| {
            let reqwest = &*reqwest;
            async move {
                let version = Version::try_from(m.version).context("Invalid version")?;

                let mut mod_id_set = HashSet::with_capacity(1);
                mod_id_set.insert(ModId {
                    owner: m.full_name.namespace().into(),
                    name: m.full_name.name().into(),
                });

                let mod_index = crate::mod_index::read_mod_index(game).await?;

                let buf = crate::mod_index::get_from_mod_index(&mod_index, &mod_id_set).await?;
                let Some(m) = buf.into_iter().next() else {
                    return Err(anyhow!("Missing mod {}", m.full_name).into());
                };

                let Some(version) = m
                    .versions
                    .iter()
                    .find(|v| v.version_number.get() == version)
                else {
                    return Err(anyhow!(
                        "Missing version {version} of mod {}-{}",
                        &*m.owner,
                        &*m.name
                    )
                    .into());
                };

                crate::profiles::install_profile_mod(
                    reqwest,
                    profile_id,
                    // this is kinda gross
                    ModMetadata {
                        name: &m.metadata.name,
                        full_name: Default::default(),
                        owner: &m.metadata.owner,
                        package_url: Default::default(),
                        donation_link: m.metadata.donation_link.as_ref().map(|s| (**s).into()),
                        date_created: m.date_created.into(),
                        date_updated: m.date_updated.into(),
                        rating_score: m.rating_score.into(),
                        is_pinned: m.is_pinned,
                        is_deprecated: m.is_deprecated,
                        has_nsfw_content: m.has_nsfw_content,
                        categories: m.categories.iter().map(|s| (**s).into()).collect(),
                        uuid4: Default::default(),
                    },
                    ModVersion {
                        name: Default::default(),
                        full_name: Default::default(),
                        description: (*version.description).into(),
                        icon: Default::default(),
                        version_number: version.version_number.get(),
                        dependencies: version.dependencies.iter().map(|s| (**s).into()).collect(),
                        download_url: Default::default(),
                        downloads: version.downloads.into(),
                        date_created: version.date_created.into(),
                        website_url: version.website_url.as_ref().map(|s| (**s).into()),
                        is_active: version.is_active,
                        uuid4: Default::default(),
                        file_size: version.file_size.into(),
                    },
                )
                .await
            }
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<()>()
        .await?;

    let profile_path = profile_path(profile_id);

    let rt = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        let local_set = tokio::task::LocalSet::new();
        rt.block_on(local_set.run_until(async move {
            (0..profile.archive.len())
                .map(|i| {
                    let mut archive = profile.archive.clone();
                    let mut target_path = profile_path.clone();
                    async move {
                        tokio::task::spawn_local(async move {
                            loop {
                                let file = archive
                                    .by_index(i)
                                    .context("Failed to open file in archive")?;

                                if file.is_dir() {
                                    break;
                                }

                                if file.is_symlink() {
                                    bail!("Symlinks are not supported");
                                }

                                let Some(path) = thunderstore::get_archive_file_path(&file)? else {
                                    break;
                                };

                                target_path.push(path);

                                tokio::fs::create_dir_all(target_path.parent().unwrap())
                                    .await
                                    .context("Unable to create target file parents")?;
                                let mut target_file = tokio::fs::File::create(&target_path)
                                    .await
                                    .with_context(|| {
                                    format!("Unable to create target file {:?}", target_path)
                                })?;

                                tokio::io::copy(
                                    &mut futures::io::AllowStdIo::new(file).compat(),
                                    &mut target_file,
                                )
                                .await
                                .context("Unable to write target file")?;

                                break;
                            }

                            Ok::<_, anyhow::Error>(())
                        })
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|r| r)
                    }
                })
                .collect::<FuturesUnordered<_>>()
                .try_collect::<()>()
                .await?;

            Ok(profile_id)
        }))
    })
    .await
    .map_err(anyhow::Error::from)?
}
