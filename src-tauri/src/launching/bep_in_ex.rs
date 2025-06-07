use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{bail, Result};
use manderrow_types::games::Game;
use tauri::AppHandle;
use tempfile::tempdir;
use uuid::Uuid;

use crate::installing::{fetch_resource_cached_by_hash_at_path, install_zip};
use crate::profiles::{profile_path, MODS_FOLDER};
use crate::Reqwest;

use super::InstructionEmitter;

fn get_url_and_hash(uses_proton: bool) -> Result<(&'static str, &'static str)> {
    macro_rules! artifact {
        ($target:literal, $hash:literal) => {
            (concat!(
                "https://github.com/manderrow/BepInEx/releases/download/v5.4.23.2%2Bbuild.18/BepInEx_",
                $target,
                "_5.4.23.2.zip"
            ), $hash)
        };
    }

    Ok(
        match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
            ("linux", "x86_64", false) => artifact!(
                "linux_x64",
                "a58d07097d87f840be5c3a86644a3580d29067a88bf1e0493bd9a5f54127e288"
            ),
            ("linux", "x86", false) => artifact!(
                "linux_x86",
                "c041863887c912f824a71cfc7508e42c4fd42904563b45becb94252c075e4cd2"
            ),
            ("macos", "x86_64", false) => artifact!(
                "macos_x64",
                "726415d1de232afa5cfb5bf7a8c1afa9fadb1cfcb5b27eae27ca5bb579bb02e8"
            ),
            ("linux", "x86_64", true) | ("windows", "x86_64", false) => artifact!(
                "win_x64",
                "3f7b79c71ba237623c1727d18e5eaa47ef314e9fd53722d575a3fc421ce9250d"
            ),
            ("linux", "x86", true) | ("windows", "x86", false) => artifact!(
                "win_x86",
                "69f7799aa2f18bf1539cfe81df967da818d6405dd4d53838139fc1575bfbf102"
            ),
            (os, arch, uses_proton) => bail!(
                "Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"
            ),
        },
    )
}

fn get_ci_url(uses_proton: bool) -> Result<&'static str> {
    macro_rules! artifact {
        ($target:literal) => {
            concat!(
                "https://github.com/manderrow/BepInEx/releases/download/ci/BepInEx_",
                $target,
                "_5.4.23.2.zip"
            )
        };
    }

    Ok(
        match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
            ("linux", "x86_64", false) => artifact!("linux_x64"),
            ("linux", "x86", false) => artifact!("linux_x86"),
            ("macos", "x86_64", false) => artifact!("macos_x64"),
            ("linux", "x86_64", true) | ("windows", "x86_64", false) => artifact!("win_x64"),
            ("linux", "x86", true) | ("windows", "x86", false) => artifact!("win_x86"),
            (os, arch, uses_proton) => bail!(
                "Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"
            ),
        },
    )
}

struct PdbArtifact {
    url: &'static str,
    hash: &'static str,
}

struct LibraryArtifact {
    url: &'static str,
    hash: &'static str,
    suffix: &'static str,
    pdb: Option<PdbArtifact>,
}

fn get_doorstop_url_and_hash(uses_proton: bool) -> Result<LibraryArtifact> {
    macro_rules! doorstop_url {
        ($artifact:literal, $suffix:literal) => {
            concat!(
                "https://github.com/manderrow/UnityDoorstop/releases/download/v4.3.0%2Bmanderrow.14/",
                $artifact,
                $suffix,
            )
        };
    }
    macro_rules! doorstop_artifact {
        ($artifact:literal, $suffix:literal, $hash:literal, pdb_hash=$pdb_hash:expr) => {
            LibraryArtifact {
                url: doorstop_url!($artifact, $suffix),
                hash: $hash,
                suffix: $suffix,
                pdb: ($pdb_hash).map(|hash| PdbArtifact {
                    url: doorstop_url!($artifact, ".pdb"),
                    hash,
                }),
            }
        };
    }

    Ok(
        match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
            ("linux", "x86_64", false) => doorstop_artifact!(
                "libUnityDoorstop_x86_64",
                ".so",
                "f2816390111a1979998e66cc1d67962aaac8c689c343b47e4c65028dbdb48e4c",
                pdb_hash=None
            ),
            ("linux", "x86", false) => todo!(),
            ("macos", "x86_64", false) => doorstop_artifact!(
                "libUnityDoorstop_x86_64",
                ".dylib",
                "e77b78e0d6abaf9838f73a6329691475ec3237a1f087e03105067472dcdff5dd",
                pdb_hash=None
            ),
            ("macos", "aarch64", false) => doorstop_artifact!(
                "libUnityDoorstop_aarch64",
                ".dylib",
                "fe0048c13ffdb867cff765a058a06151bfb9bf8096329fe4063cd61d9fcb5149",
                pdb_hash=None
            ),
            ("linux", "x86_64", true) | ("windows", "x86_64", false) => doorstop_artifact!(
                "UnityDoorstop_x86_64",
                ".dll",
                "5fac088052e6256f362d4b951c46d2c017900a9655c0e9464aacb4664ba636c8",
                pdb_hash=Some("f85f22eaf57f0f3529323b51e170cd433e22f3c9f7e9046a999f8cbdfe044f7b")
            ),
            ("linux", "x86", true) | ("windows", "x86", false) => doorstop_artifact!(
                "UnityDoorstop_x86",
                ".dll",
                "fbafa8e3fc2b9371737d2b45eb9e8b7eb40573eaa6eb515738d7d7932565f784",
                pdb_hash=Some("9e9af7eb20484aac7d90295bb640cb724ef6c2dc807275e959dd246657af869b")
            ),
            (os, arch, uses_proton) => bail!(
                "Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"
            ),
        },
    )
}

#[derive(Clone, Copy)]
pub enum BepInExVersion {
    Stable,
    Ci,
}

/// Returns the absolute path to the BepInEx installation. If BepInEx has not yet been
/// installed, this function will take care of that before returning.
pub async fn get_bep_in_ex_path(
    log: &slog::Logger,
    version: BepInExVersion,
    uses_proton: bool,
) -> Result<PathBuf> {
    let (url, cache, path) = match version {
        BepInExVersion::Stable => {
            let (url, hash) = get_url_and_hash(uses_proton)?;
            (
                url,
                Some(crate::installing::CacheOptions::by_hash(hash)),
                crate::launching::LOADERS_DIR.join(hash),
            )
        }
        BepInExVersion::Ci => (
            get_ci_url(uses_proton)?,
            // TODO: maybe cache by etag or something?
            None,
            crate::launching::LOADERS_DIR.join("ci"),
        ),
    };

    install_zip(
        // TODO: communicate via IPC
        None,
        log,
        &Reqwest(reqwest::Client::new()),
        url,
        cache,
        &path,
        None,
    )
    .await?
    .finish(log)
    .await?;

    Ok(path)
}

pub async fn emit_instructions(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    mut em: InstructionEmitter<'_>,
    game: &Game<'_>,
    profile_id: Uuid,
    version: BepInExVersion,
    doorstop_path: Option<PathBuf>,
    uses_proton: bool,
) -> anyhow::Result<()> {
    let bep_in_ex = get_bep_in_ex_path(log, version, false).await?;

    let profile_path = profile_path(profile_id);

    let temp_dir = tempdir()?.into_path();

    em.set_var("BEPINEX_CONFIGS", profile_path.join("config"));
    em.set_var("BEPINEX_PLUGINS", profile_path.join(MODS_FOLDER));
    em.set_var("BEPINEX_PATCHER_PLUGINS", profile_path.join("patchers"));
    // TODO: should this point to a "persistent" cache directory, and should it be per-profile or shared?
    em.set_var("BEPINEX_CACHE", temp_dir.join("cache"));
    // enables the logging we expect from our fork of BepInEx
    em.set_var("BEPINEX_STANDARD_LOG", "");

    let target_assembly = if uses_proton {
        let mut buf = OsString::from("Z:");
        const SUFFIX: &str = "/BepInEx/core/BepInEx.Preloader.dll";
        buf.reserve_exact(bep_in_ex.as_os_str().len() + SUFFIX.len());
        buf.push(bep_in_ex.as_os_str());
        buf.push(SUFFIX);
        PathBuf::from(buf)
    } else {
        let mut p = bep_in_ex.clone();
        p.push("BepInEx");
        p.push("core");
        p.push("BepInEx.Preloader.dll");
        p
    };

    // note for the future: any paths provided to UnityDoorstop must be absolute.
    em.set_var("DOORSTOP_ENABLED", "1");
    em.set_var("DOORSTOP_TARGET_ASSEMBLY", &target_assembly);
    em.set_var("DOORSTOP_IGNORE_DISABLED_ENV", "0");
    // specify these only if they have values
    // em.set_var("DOORSTOP_MONO_DLL_SEARCH_PATH_OVERRIDE", "");
    em.set_var("DOORSTOP_MONO_DEBUG_ENABLED", "0");
    em.set_var("DOORSTOP_MONO_DEBUG_ADDRESS", "127.0.0.1:10000");
    em.set_var("DOORSTOP_MONO_DEBUG_SUSPEND", "0");
    // specify these only if they have values
    // em.set_var("DOORSTOP_CLR_CORLIB_DIR", "");
    // em.set_var("DOORSTOP_CLR_RUNTIME_CORECLR_PATH", "");
    // em.set_var("DOORSTOP_BOOT_CONFIG_OVERRIDE", "/path/to/boot.config");

    let doorstop_path = match doorstop_path {
        Some(t) => t,
        None => {
            let LibraryArtifact {
                url,
                hash,
                suffix,
                pdb,
            } = get_doorstop_url_and_hash(uses_proton)?;

            let mut path = manderrow_paths::cache_dir().join(hash);

            match tokio::fs::create_dir(&path).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }

            path.push(hash);

            if let Some(pdb) = pdb {
                path.as_mut_os_string().push(".pdb");

                fetch_resource_cached_by_hash_at_path(
                    app,
                    log,
                    &Reqwest(reqwest::Client::new()),
                    pdb.url,
                    pdb.hash,
                    &path,
                    None,
                )
                .await?;

                let len = path.as_mut_os_string().len();
                path.as_mut_os_string().truncate(len - 4);
            }

            path.as_mut_os_string().push(suffix);

            fetch_resource_cached_by_hash_at_path(
                app,
                log,
                &Reqwest(reqwest::Client::new()),
                url,
                hash,
                &path,
                None,
            )
            .await?;

            path
        }
    };

    em.load_library(doorstop_path);

    Ok(())
}
