use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use anyhow::{bail, Result};
use tempfile::tempdir;
use uuid::Uuid;

use crate::installing::{fetch_resource_cached_by_hash, install_zip};
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

fn get_doorstop_url_and_hash(
    uses_proton: bool,
) -> Result<(&'static str, &'static str, &'static str)> {
    macro_rules! doorstop_artifact {
        ($artifact:literal, $suffix:literal, $hash:literal) => {
            (concat!(
                "https://github.com/manderrow/UnityDoorstop/releases/download/v4.3.0%2Bmanderrow.9/",
                $artifact,
                $suffix
            ), $hash, $suffix)
        };
    }

    Ok(
        match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
            ("linux", "x86_64", false) => doorstop_artifact!(
                "libUnityDoorstop",
                ".so",
                "845e0494a44c88c576765c1ce850a7f883ce2253948c4617c0cffee1635853a6"
            ),
            ("linux", "x86", false) => todo!(),
            ("macos", "x86_64", false) => doorstop_artifact!(
                "libUnityDoorstop",
                ".dylib",
                "8e2ce9c37149c5384a6a18e40ad0d23e1ac750925acbc6b5ba612f6c2f4f1a28"
            ),
            ("linux", "x86_64", true) | ("windows", "x86_64", false) => doorstop_artifact!(
                "UnityDoorstop",
                ".dll",
                "efcea495b5191f3931f8be21b001b2c851e988614f5dce155546e007ef187cf1"
            ),
            ("linux", "x86", true) | ("windows", "x86", false) => todo!(),
            (os, arch, uses_proton) => bail!(
                "Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"
            ),
        },
    )
}

/// Returns the absolute path to the BepInEx installation. If BepInEx has not yet been
/// installed, this function will take care of that before returning.
pub async fn get_bep_in_ex_path(log: &slog::Logger, uses_proton: bool) -> Result<PathBuf> {
    const USE_CI: bool = false;
    let (url, cache, path) = if USE_CI {
        (
            get_ci_url(uses_proton)?,
            // TODO: maybe cache by etag or something?
            None,
            crate::launching::LOADERS_DIR.join("ci"),
        )
    } else {
        let (url, hash) = get_url_and_hash(uses_proton)?;
        (
            url,
            Some(crate::installing::CacheOptions::by_hash(hash)),
            crate::launching::LOADERS_DIR.join(hash),
        )
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
    log: &slog::Logger,
    mut em: InstructionEmitter<'_>,
    game: &str,
    profile_id: Uuid,
    doorstop_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let bep_in_ex = get_bep_in_ex_path(log, false).await?;

    let profile_path = profile_path(profile_id);

    let uses_proton = crate::stores::steam::proton::uses_proton(log, game).await?;

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

    let (doorstop_url, doorstop_hash, doorstop_suffix) = get_doorstop_url_and_hash(uses_proton)?;

    let doorstop_path = match doorstop_path {
        Some(t) => t,
        None => {
            fetch_resource_cached_by_hash(
                // TODO: communicate via IPC
                None,
                log,
                &Reqwest(reqwest::Client::new()),
                doorstop_url,
                doorstop_hash,
                doorstop_suffix,
                None,
            )
            .await?
        }
    };

    em.load_library(doorstop_path);

    Ok(())
}
