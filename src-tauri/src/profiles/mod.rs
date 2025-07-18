pub mod commands;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{anyhow, ensure, Context as _, Result};
use futures_util::stream::FuturesOrdered;
use futures_util::StreamExt as _;
use manderrow_paths::local_data_dir;
use manderrow_types::mods::{ModAndVersion, ModId, ModMetadata, ModSpec, ModVersion};
use manderrow_types::util::serde::IgnoredAny;
use packed_semver::Version;
use parking_lot::Mutex;
use slog::{debug, error};
use smol_str::SmolStr;
use tauri::AppHandle;
use uuid::Uuid;

use crate::installing::{
    create_dir_if_not_exists, install_folder, prepare_install_zip, uninstall_package, StagedPackage,
};
use crate::util::{hyphenated_uuid, IoErrorKindExt as _};
use crate::{tasks, Reqwest};

pub static PROFILES_DIR: LazyLock<PathBuf> = LazyLock::new(|| local_data_dir().join("profiles"));

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: SmolStr,
    pub game: SmolStr,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ProfileWithId {
    pub id: Uuid,
    #[serde(flatten)]
    pub metadata: Profile,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadProfileError {
    #[error("failed to read profile.json: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse profile.json: {0}")]
    Decoding(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum WriteProfileError {
    #[error("failed to write profile.json: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to encode profile.json: {0}")]
    Encoding(#[from] serde_json::Error),
}

pub async fn read_profile_file(path: &Path) -> Result<Profile, ReadProfileError> {
    Ok(serde_json::from_slice(&tokio::fs::read(path).await?)?)
}

pub async fn write_profile_file(path: &Path, metadata: &Profile) -> Result<(), WriteProfileError> {
    tokio::fs::write(path, serde_json::to_vec(metadata)?).await?;
    Ok(())
}

pub async fn read_profile(id: Uuid) -> Result<Profile, ReadProfileError> {
    let mut path = profile_path(id);
    path.push("profile.json");
    read_profile_file(&path).await
}

pub async fn write_profile(id: Uuid, metadata: &Profile) -> Result<(), WriteProfileError> {
    let mut path = profile_path(id);
    path.push("profile.json");
    write_profile_file(&path, metadata).await
}

pub fn profile_path(id: Uuid) -> PathBuf {
    PROFILES_DIR.join(hyphenated_uuid!(id))
}

pub async fn get_profiles() -> Result<Vec<ProfileWithId>> {
    let log = slog_scope::logger();

    let mut profiles = Vec::new();
    let mut iter = match tokio::fs::read_dir(&*PROFILES_DIR).await {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(Vec::new()),
        Err(e) => return Err(e).context("Failed to read profiles directory")?,
    };
    while let Some(e) = iter
        .next_entry()
        .await
        .context("Failed to read profiles directory")?
    {
        let mut path = e.path();
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if id.len() != 36 {
            continue;
        }
        let Ok(id) = Uuid::try_parse(id) else {
            continue;
        };
        path.push("profile.json");
        let metadata = match read_profile_file(&path).await {
            Ok(t) => t,
            Err(ReadProfileError::Io(e)) if e.is_not_found() => continue,
            Err(e) => {
                error!(log, "Unable to read profile metadata from {path:?}: {e}");
                continue;
            }
        };
        profiles.push(ProfileWithId { id, metadata });
    }
    Ok(profiles)
}

pub async fn create_profile(game: SmolStr, name: SmolStr) -> Result<Uuid> {
    tokio::fs::create_dir_all(&*PROFILES_DIR)
        .await
        .context("Failed to create profiles directory")?;
    let id = Uuid::new_v4();
    let mut path = profile_path(id);
    tokio::fs::create_dir(&path)
        .await
        .context("Failed to create profile directory")?;
    path.push("profile.json");
    write_profile_file(
        &path,
        &Profile {
            name,
            game,
            pinned: false,
        },
    )
    .await
    .context("Failed to write profile metadata")?;
    Ok(id)
}

pub async fn delete_profile(id: Uuid) -> Result<()> {
    let path = profile_path(id);
    tokio::fs::remove_dir_all(&path)
        .await
        .context("Failed to delete profile directory")?;
    Ok(())
}

pub const MODS_FOLDER: &str = "mods";
pub const CONFIG_FOLDER: &str = "config";
pub const PATCHERS_FOLDER: &str = "patchers";

const MANIFEST_FILE_NAME: &str = "manderrow_mod.json";

pub async fn get_profile_mods(id: Uuid) -> Result<tauri::ipc::Response> {
    let mut path = profile_path(id);

    path.push(MODS_FOLDER);

    let mut iter = match tokio::fs::read_dir(&path).await {
        Ok(t) => t,
        Err(e) if e.is_not_found() => return Ok(tauri::ipc::Response::new("[]".to_owned())),
        Err(e) => return Err(anyhow::Error::from(e).into()),
    };
    let mut tasks = FuturesOrdered::new();
    while let Some(e) = iter.next_entry().await.map_err(anyhow::Error::from)? {
        if e.file_type().await.map_err(anyhow::Error::from)?.is_dir() {
            let mut path = path.clone();
            tasks.push_back(tokio::task::spawn(async move {
                path.push(e.file_name());
                path.push(MANIFEST_FILE_NAME);
                match tokio::fs::read_to_string(&path).await {
                    Ok(t) => Ok(Some(t)),
                    Err(e) if e.is_not_found() => return Ok(None),
                    Err(e) => {
                        return Err(anyhow::Error::from(e)
                            .context(format!("Failed to read mod manifest {path:?}")))
                    }
                }
            }));
        }
    }
    let mut buf = "[".to_owned();
    let mut first = true;
    while let Some(r) = tasks.next().await {
        if let Some(m) = r.map_err(anyhow::Error::from)?? {
            if first {
                first = false;
            } else {
                buf.push(',');
            }
            buf.push_str(&m);
        }
    }
    buf.push(']');
    Ok(tauri::ipc::Response::new(buf))
}

pub async fn install_profile_mod(
    app: &AppHandle,
    reqwest: &Reqwest,
    id: Uuid,
    r#mod: ModMetadata<'_>,
    version: ModVersion<'_>,
    task_id: Option<tasks::Id>,
) -> Result<()> {
    let log = slog_scope::logger();

    if r#mod.owner == "BepInEx" && r#mod.name == "BepInExPack" {
        return Err(anyhow!(
            "BepInEx pack is managed by manderrow and will be installed automatically if required"
        ));
    }

    let mut profile_path = profile_path(id);
    profile_path.push("profile.json");
    let game = read_profile_file(&profile_path).await?.game;
    profile_path.pop();

    let mod_index = crate::mod_index::read_mod_index(&game).await?;

    let seen = Mutex::new(HashMap::new());
    install_profile_mod_inner(
        &log,
        app,
        reqwest,
        id,
        &profile_path,
        &mod_index,
        r#mod.owner,
        r#mod.name,
        version.version_number,
        task_id,
        &seen,
    )
    .await?;

    for (id, m) in seen.into_inner() {
        debug!(log, "committing installation of {}-{}", id, m.version);
        for transaction in m.transactions {
            transaction.commit(&log).await?;
        }
    }

    Ok(())
}

struct InstallingMod {
    version: Version,
    transactions: Vec<crate::installing::ReplaceTransaction>,
}

/// `game` must match the profile's game.
///
/// As currently implemented, this may return before the mod is actually installed if it is being
/// installed by another invocation concurrently and marked as such in `seen`.
async fn install_profile_mod_inner<'a, 'b>(
    log: &slog::Logger,
    app: &AppHandle,
    reqwest: &Reqwest,
    id: Uuid,
    profile_path: &Path,
    mod_index: &'a crate::mod_index::ModIndexReadGuard,
    mod_owner: &'a str,
    mod_name: &'a str,
    mod_version: Version,
    task_id: Option<tasks::Id>,
    seen: &Mutex<HashMap<ModId<'a>, InstallingMod>>,
) -> Result<()> {
    let mod_id = ModId {
        owner: mod_owner.into(),
        name: mod_name.into(),
    };

    // must not hold the lock across an await
    if seen
        .lock()
        .try_insert(
            mod_id,
            InstallingMod {
                version: mod_version,
                transactions: Vec::new(),
            },
        )
        .is_err()
    {
        // FIXME: check semver compatibility
        return Ok(());
    }

    let Some(m) = crate::mod_index::get_one_from_mod_index(
        mod_index,
        ModId {
            owner: mod_owner.into(),
            name: mod_name.into(),
        },
    )
    .await?
    else {
        return Err(anyhow!("Missing dependency {}", mod_id));
    };
    let Some(version) = m
        .versions
        .iter()
        .find(|v| v.version_number.get() == mod_version)
    else {
        return Err(anyhow!(
            "Missing version {} of dependency {}",
            mod_version,
            mod_id
        ));
    };

    let mut mod_folder_path = profile_path.join(MODS_FOLDER);
    let mut patchers_folder_path = profile_path.join(PATCHERS_FOLDER);

    create_dir_if_not_exists(&patchers_folder_path)
        .await
        .context("failed to create profile patchers folder")?;

    push_mod_folder(&mut mod_folder_path, mod_owner, mod_name);
    push_mod_folder(&mut patchers_folder_path, mod_owner, mod_name);

    let url = format!(
        "https://gcdn.thunderstore.io/live/repository/packages/{}-{}-{}.zip",
        mod_owner, mod_name, version.version_number
    );

    debug!(
        log,
        "Installing mod from {url:?} to profile {id} at {mod_folder_path:?}"
    );

    futures_util::future::try_join_all(version.dependencies.iter().map(
        |dep: &'a manderrow_types::util::rkyv::ArchivedInternedString| async move {
            // you get a really nasty lifetime error if you forget the `.map_err(...)`
            let mod_spec = ModSpec::<'a>::from_str(&*dep).map_err(|e| anyhow!("{e}"))?;

            if &*mod_spec.id().owner == "BepInEx" && &*mod_spec.id().name == "BepInExPack" {
                return Ok(());
            }

            install_profile_mod_inner(
                log,
                app,
                reqwest,
                id,
                profile_path,
                mod_index,
                mod_spec.id().owner.0,
                mod_spec.id().name.0,
                mod_spec.version,
                None,
                seen,
            )
            .await
        },
    ))
    .await?;

    let mod_temp_dir = prepare_install_zip(
        Some(app),
        &log,
        reqwest,
        format!("{mod_owner}-{mod_name}-{mod_version}"),
        &url,
        Some(crate::installing::CacheOptions::by_url()),
        &mod_folder_path,
        task_id,
    )
    .await?;

    {
        let mut entries = Vec::new();
        let mut iter = tokio::fs::read_dir(mod_temp_dir.path()).await?;
        while let Some(e) = iter.next_entry().await? {
            entries.push(e.path());
        }
        debug!(log, "prepared mod package for installation: {:?}", entries);
    }

    let patchers_temp_dir =
        crate::installing::generate_temp_path(&patchers_folder_path, ".tmp-").await?;
    let patchers_og_dir = mod_temp_dir.path().join(PATCHERS_FOLDER);
    let patchers_staged: Option<StagedPackage>;
    match tokio::fs::rename(&patchers_og_dir, &patchers_temp_dir).await {
        Ok(()) => {
            patchers_staged =
                Some(install_folder(&log, &patchers_temp_dir, &patchers_folder_path).await?);

            ensure!(
                tokio::fs::try_exists(patchers_staged.as_ref().unwrap().path()).await?,
                "must exist after patchers install"
            );
        }
        Err(e) if e.is_not_found() => {
            patchers_staged = None;
        }
        Err(e) => return Err(e.into()),
    }

    let staged = install_folder(&log, mod_temp_dir.path(), &mod_folder_path).await?;
    staged.check_with_temp_dir(&mod_temp_dir);

    let mods_staged = StagedPackage {
        target: &mod_folder_path,
        source: crate::installing::StagedPackageSource::TempDir(mod_temp_dir),
    };

    if let Some(ref patchers_staged) = patchers_staged {
        ensure!(
            tokio::fs::try_exists(patchers_staged.path()).await?,
            "must exist after mods install"
        );
    }

    // TODO: create a dedicated ModManifest type that is saved locally, with some fields stripped (all IgnoredAny, and some others)
    tokio::task::block_in_place(|| {
        serde_json::to_writer(
            std::io::BufWriter::new(std::fs::File::create(
                mods_staged.path().join(MANIFEST_FILE_NAME),
            )?),
            &ModAndVersion {
                r#mod: ModMetadata {
                    name: &m.name,
                    full_name: IgnoredAny,
                    owner: &m.owner,
                    package_url: IgnoredAny,
                    donation_link: m.donation_link.as_ref().map(|s| SmolStr::from(&**s)),
                    date_created: m.date_created.into(),
                    // TODO: don't save this locally?
                    date_updated: m.date_updated.into(),
                    // TODO: don't save this locally
                    rating_score: m.rating_score.into(),
                    // TODO: don't save this locally
                    is_pinned: m.is_pinned,
                    is_deprecated: m.is_deprecated,
                    has_nsfw_content: m.has_nsfw_content,
                    categories: m.categories.iter().map(|s| SmolStr::from(&**s)).collect(),
                    uuid4: IgnoredAny,
                },
                version: ModVersion {
                    name: IgnoredAny,
                    full_name: IgnoredAny,
                    description: SmolStr::from(&*version.description),
                    icon: IgnoredAny,
                    version_number: version.version_number.get(),
                    dependencies: version.dependencies.iter().map(|s| s.into()).collect(),
                    download_url: IgnoredAny,
                    // TODO: don't save this locally
                    downloads: version.downloads.into(),
                    date_created: version.date_created.into(),
                    website_url: version.website_url.as_ref().map(|s| SmolStr::from(&**s)),
                    is_active: version.is_active,
                    uuid4: IgnoredAny,
                    file_size: version.file_size.into(),
                },
            },
        )?;
        Ok::<_, anyhow::Error>(())
    })?;

    let patchers_transaction = if let Some(patchers_staged) = patchers_staged {
        Some(patchers_staged.apply(&log).await?)
    } else {
        None
    };
    let mods_transaction = mods_staged.apply(&log).await?;

    // must not hold the lock across an await
    let mut seen = seen.lock();
    let transactions = &mut seen.get_mut(&mod_id).unwrap().transactions;

    if let Some(transaction) = patchers_transaction {
        transactions.push(transaction);
    }
    transactions.push(mods_transaction);

    Ok(())
}

fn push_mod_folder(path: &mut PathBuf, owner: &str, name: &str) {
    path.push(owner);
    path.as_mut_os_string().push("-");
    path.as_mut_os_string().push(name);
}

pub async fn uninstall_profile_mod(id: Uuid, owner: &str, name: &str) -> Result<()> {
    let log = slog_scope::logger();

    let mut path = profile_path(id);

    for folder in [MODS_FOLDER, PATCHERS_FOLDER] {
        path.push(folder);
        push_mod_folder(&mut path, owner, name);

        // remove the manifest so it isn't left over after uninstalling the package
        path.push(MANIFEST_FILE_NAME);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(e) if e.is_not_found() => {}
            Err(e) => {
                return Err(anyhow::Error::from(e)
                    .context(format!("Failed to remove manifest file at {path:?}")))
            }
        }
        path.pop();

        // keep_changes is true so that configs and any other changes are
        // preserved. Zero-risk uninstallation!
        uninstall_package(&log, &path, true).await?;
        path.pop();
        path.pop();
    }
    Ok(())
}
