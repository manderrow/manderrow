//! Importing modpacks as profiles.

pub mod thunderstore;

use anyhow::{anyhow, bail, Context};
use futures_util::stream::FuturesUnordered;
use futures_util::TryStreamExt;
use manderrow_types::mods::ModId;
use packed_semver::Version;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use uuid::Uuid;

use crate::app::AppHandle;
use crate::mod_index::fetch_mod_index;
use crate::profiles::profile_path;
use crate::tasks::{TaskBuilder, TaskError, TaskHandle};
use crate::{tasks, CommandError, Reqwest};

/// What a modpack would make of a profile.
#[derive(Debug)]
pub struct Modpack {
    pub name: String,
    pub mods: Vec<ModpackMod>,
    /// The paths of the files it adds to the profile, as its mods' configuration.
    pub files: Vec<String>,
}

/// A version of a mod in a modpack.
#[derive(Debug)]
pub struct ModpackMod {
    pub owner: String,
    pub name: String,
    pub version: String,
}

/// Looks up a modpack that's been shared as a Thunderstore profile code.
pub async fn preview_thunderstore_modpack(
    app: &AppHandle,
    reqwest: &Reqwest,
    thunderstore_id: Uuid,
    task_id: tasks::Id,
) -> Result<Modpack, CommandError> {
    let log = slog_scope::logger();

    let mut profile =
        thunderstore::lookup_profile(Some(app), &log, reqwest, thunderstore_id, Some(task_id))
            .await?;

    let mods = profile
        .manifest
        .mods
        .iter()
        .map(|m| {
            let (owner, name) = m.full_name.components();
            ModpackMod {
                owner: owner.to_owned(),
                name: name.to_owned(),
                version: m.version.to_string(),
            }
        })
        .collect();

    let mut files = Vec::with_capacity(profile.archive.len());

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

        files.push(path);
    }

    Ok(Modpack {
        name: profile.manifest.profile_name,
        mods,
        files,
    })
}

/// Tells of the task of a mod's installation, by the mod's owner and name, as it starts.
pub type OnModTask = dyn Fn(&str, &str, tasks::Id) + Sync;

/// Imports a modpack that's been shared as a Thunderstore profile code as a new profile, and
/// returns its id. Importing onto an existing profile isn't supported yet.
pub async fn import_thunderstore_modpack(
    app: &AppHandle,
    reqwest: &Reqwest,
    thunderstore_id: Uuid,
    game: &str,
    profile_id: Option<Uuid>,
    on_mod_task: &OnModTask,
    task_id: tasks::Id,
) -> Result<Uuid, CommandError> {
    if profile_id.is_some() {
        return Err(anyhow!("Importing over existing profiles is not yet supported").into());
    }

    let log = slog_scope::logger();

    TaskBuilder::with_id(
        task_id,
        format!("Import modpack thunderstore:{thunderstore_id}"),
    )
    .kind(tasks::Kind::Aggregate)
    .progress_unit(tasks::ProgressUnit::Bytes)
    .run_with_handle(Some(app), |handle| async move {
        fetch_mod_index(
            Some(app),
            &app.state(),
            game,
            false,
            Some(handle.allocate_dependency(app)),
        )
        .await?;

        let profile = thunderstore::lookup_profile(
            Some(app),
            &log,
            reqwest,
            thunderstore_id,
            Some(handle.allocate_dependency(app)),
        )
        .await?;

        let (profile_id, is_new_profile) = match profile_id {
            Some(profile_id) => (profile_id, false),
            None => (
                crate::profiles::create_profile(
                    game.into(),
                    profile.manifest.profile_name.as_str().into(),
                )
                .await?,
                true,
            ),
        };

        if let Err(e) = import_onto_profile(
            app,
            reqwest,
            game,
            profile,
            profile_id,
            on_mod_task,
            handle,
        )
        .await
        {
            if is_new_profile {
                crate::profiles::delete_profile(profile_id).await?;
            }
            return Err(e);
        }

        Ok((None, profile_id))
    })
    .await
    .map_err(|e: TaskError<anyhow::Error>| anyhow::Error::from(e).into())
}

async fn import_onto_profile(
    app: &AppHandle,
    reqwest: &Reqwest,
    game: &str,
    profile: thunderstore::Profile,
    profile_id: Uuid,
    on_mod_task: &OnModTask,
    handle: TaskHandle,
) -> Result<(), anyhow::Error> {
    profile
        .manifest
        .mods
        .iter()
        .map(|m| {
            async move {
                let version = Version::try_from(m.version).context("Invalid version")?;

                let mod_index = crate::mod_index::read_mod_index(game).await?;

                let Some(m) = crate::mod_index::get_one_from_mod_index(
                    &mod_index,
                    ModId {
                        owner: m.full_name.namespace().into(),
                        name: m.full_name.name().into(),
                    },
                )
                .await?
                else {
                    return Err(anyhow!("Missing mod {}", m.full_name));
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
                    );
                };

                let sub_task_id = handle.allocate_dependency(app);
                on_mod_task(&m.owner, &m.name, sub_task_id);
                crate::profiles::install_profile_mod(
                    app,
                    reqwest,
                    profile_id,
                    &m.owner,
                    &m.name,
                    version.version_number.get(),
                    sub_task_id,
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
                            let file = archive
                                .by_index(i)
                                .context("Failed to open file in archive")?;

                            if file.is_dir() {
                                return Ok(());
                            }

                            if file.is_symlink() {
                                bail!("Symlinks are not supported");
                            }

                            let Some(path) = thunderstore::get_archive_file_path(&file)? else {
                                return Ok(());
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
                                &mut futures_util::io::AllowStdIo::new(file).compat(),
                                &mut target_file,
                            )
                            .await
                            .context("Unable to write target file")?;

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

            Ok(())
        }))
    })
    .await?
}
