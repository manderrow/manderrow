use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use anyhow::{bail, Context as _, Result};
use slog::debug;
use tempfile::tempdir;
use uuid::Uuid;

use crate::games::games_by_id;
use crate::installing::{fetch_resource_cached_by_hash, install_file, install_zip};
use crate::profiles::{profile_path, MODS_FOLDER};
use crate::stores::steam::paths::resolve_steam_app_install_directory;
use crate::stores::steam::proton::{ensure_wine_will_load_dll_override, uses_proton};
use crate::Reqwest;

pub trait CommandBuilder {
    fn env(&mut self, key: impl AsRef<str>, value: impl AsRef<OsStr>);

    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>);

    fn arg(&mut self, arg: impl AsRef<std::ffi::OsStr>);
}

fn get_url_and_hash(uses_proton: bool) -> Result<(&'static str, &'static str)> {
    macro_rules! artifact {
        ($target:literal, $hash:literal) => {
            (concat!(
                "https://github.com/manderrow/BepInEx/releases/download/v5.4.23.2%2Bbuild.17/BepInEx_",
                $target,
                "_5.4.23.2.zip"
            ), $hash)
        };
    }

    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("linux", "x86_64", false) => artifact!("linux_x64", "19ad62133fe1c548bad68991d6566bce86425a67e9537d05d0feb2cd3513d28d"),
        ("linux", "x86", false) => artifact!("linux_x86", "ccce243cac71858584d5391c645ae25a866bfcb4112c3885235fce21fb444c2d"),
        ("macos", "x86_64", false) => artifact!("macos_x64", "8b133359aafd775f900ab4310e008100d44202506140f19d4f2a950a92c8f44c"),
        ("linux", "x86_64", true) | ("windows", "x86_64", false) => artifact!("win_x64", "6f9e5436f25ab5802695e616077915ccefa1ff0d7e5fab09522e66ee06e04cb2"),
        ("linux", "x86", true) | ("windows", "x86", false) => artifact!("win_x86", "9238ad80c275a605250804fa7d26f2e47fd417953600fb16012c36f0b1d7de13"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
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

    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("linux", "x86_64", false) => artifact!("linux_x64"),
        ("linux", "x86", false) => artifact!("linux_x86"),
        ("macos", "x86_64", false) => artifact!("macos_x64"),
        ("linux", "x86_64", true) | ("windows", "x86_64", false) => artifact!("win_x64"),
        ("linux", "x86", true) | ("windows", "x86", false) => artifact!("win_x86"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
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

    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("linux", "x86_64", false) => doorstop_artifact!("libUnityDoorstop", ".so", "845e0494a44c88c576765c1ce850a7f883ce2253948c4617c0cffee1635853a6"),
        ("linux", "x86", false) => todo!(),
        ("macos", "x86_64", false) => doorstop_artifact!("libUnityDoorstop", ".dylib", "8e2ce9c37149c5384a6a18e40ad0d23e1ac750925acbc6b5ba612f6c2f4f1a28"),
        ("linux", "x86_64", true) | ("windows", "x86_64", false) => doorstop_artifact!("UnityDoorstop", ".dll", "efcea495b5191f3931f8be21b001b2c851e988614f5dce155546e007ef187cf1"),
        ("linux", "x86", true) | ("windows", "x86", false) => todo!(),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
}

/// Returns the absolute path to the BepInEx installation. If BepInEx has not yet been
/// installed, this function will take care of that before returning.
pub async fn get_bep_in_ex_path(log: &slog::Logger, uses_proton: bool) -> Result<PathBuf> {
    const USE_CI: bool = false;
    let (url, cache, path) = if USE_CI {
        (
            get_ci_url(uses_proton)?,
            Some(crate::installing::CacheOptions::by_url()),
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

pub async fn configure_command(
    log: &slog::Logger,
    command: &mut impl CommandBuilder,
    game: &str,
    profile_id: Uuid,
    doorstop_path: Option<PathBuf>,
    legacy_doorstop: bool,
) -> anyhow::Result<()> {
    let steam_metadata = games_by_id()?
        .get(game)
        .context("No such game")?
        .store_platform_metadata
        .iter()
        .find_map(|m| m.steam_or_direct())
        .context("Unsupported store platform")?;

    let uses_proton = uses_proton(log, steam_metadata.id).await?;

    let bep_in_ex = get_bep_in_ex_path(log, uses_proton).await?;

    let profile_path = profile_path(profile_id);

    let temp_dir = tempdir()?.into_path();

    command.env("BEPINEX_CONFIGS", profile_path.join("config"));
    command.env("BEPINEX_PLUGINS", profile_path.join(MODS_FOLDER));
    command.env("BEPINEX_PATCHER_PLUGINS", profile_path.join("patchers"));
    // TODO: should this point to a "persistent" cache directory, and should it be per-profile or shared?
    command.env("BEPINEX_CACHE", temp_dir.join("cache"));
    // enables the logging we expect from our fork of BepInEx
    command.env("BEPINEX_STANDARD_LOG", "");

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
    command.env("DOORSTOP_ENABLED", "1");
    command.env("DOORSTOP_TARGET_ASSEMBLY", &target_assembly);
    command.env("DOORSTOP_IGNORE_DISABLED_ENV", "0");
    // specify these only if they have values
    // command.env("DOORSTOP_MONO_DLL_SEARCH_PATH_OVERRIDE", "");
    command.env("DOORSTOP_MONO_DEBUG_ENABLED", "0");
    command.env("DOORSTOP_MONO_DEBUG_ADDRESS", "127.0.0.1:10000");
    command.env("DOORSTOP_MONO_DEBUG_SUSPEND", "0");
    // specify these only if they have values
    // command.env("DOORSTOP_CLR_CORLIB_DIR", "");
    // command.env("DOORSTOP_CLR_RUNTIME_CORECLR_PATH", "");
    // command.env("DOORSTOP_BOOT_CONFIG_OVERRIDE", "/path/to/boot.config");

    let (doorstop_url, doorstop_hash, doorstop_suffix) = get_doorstop_url_and_hash(uses_proton)?;

    if cfg!(windows) || uses_proton {
        if uses_proton {
            // TODO: don't overwrite anything without checking with the user
            //       via a doctor's note.
            ensure_wine_will_load_dll_override(log, steam_metadata.id, "winhttp").await?;
        }

        let doorstop_install_target = resolve_steam_app_install_directory(steam_metadata.id)
            .await?
            .join("winhttp.dll");
        if let Some(doorstop_path) = doorstop_path {
            tokio::fs::copy(doorstop_path, &doorstop_install_target).await?;
        } else {
            install_file(
                // TODO: communicate via IPC
                None,
                log,
                &Reqwest(reqwest::Client::new()),
                doorstop_url,
                // suffix is unnecessary here
                Some(crate::installing::CacheOptions::by_hash(doorstop_hash)),
                &doorstop_install_target,
                None,
            )
            .await?;
        }

        if legacy_doorstop {
            command.args(["--doorstop-enable", "true"]);

            command.arg("--doorstop-target-assembly");
            command.arg(&target_assembly);

            command.args(["--doorstop-mono-debug-enabled", "false"]);
            command.args(["--doorstop-mono-debug-address", "127.0.0.1:10000"]);
            command.args(["--doorstop-mono-debug-suspend", "false"]);
            // specify these only if they have values
            // especially --doorstop-mono-dll-search-path-override, which will
            // cause the doorstop to fail if given an empty string
            // command.args(["--doorstop-mono-dll-search-path-override", ""]);
            // command.args(["--doorstop-clr-corlib-dir", ""]);
            // command.args(["--doorstop-clr-runtime-coreclr-path", ""]);
        }
    } else {
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

        // TODO: test on Linux and verify this is unnecessary, then delete
        //         for var in ["LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"] {
        //             let base = std::env::var_os(var).unwrap_or_else(OsString::new);
        //             let mut buf = bep_in_ex.as_os_str().to_owned();
        //             if !base.is_empty() {
        //                 buf.push(":");
        //                 buf.push(base);
        //             }
        //
        //             command.env(var, buf);
        //         }

        const VAR: &str = if cfg!(target_os = "macos") {
            "DYLD_INSERT_LIBRARIES"
        } else {
            "LD_PRELOAD"
        };
        let base = std::env::var_os(VAR).unwrap_or_else(OsString::new);
        let mut buf = doorstop_path.clone().into_os_string();
        if !base.is_empty() {
            buf.push(":");
            buf.push(base);
        }

        debug!(log, "Injecting {VAR} {buf:?}");

        command.env(VAR, buf);
    }

    Ok(())
}
