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
                "https://github.com/mpfaff/BepInEx/releases/download/v5.4.23.2%2Bbuild.16/BepInEx_",
                $target,
                "_5.4.23.2.zip"
            ), $hash)
        };
    }

    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("linux", "x86_64", false) => artifact!("linux_x64", "e4ab751df846565012f75979b55ee4bc0b8232c7cb7227bea073a3e1dddeaf95"),
        ("linux", "x86", false) => artifact!("linux_x86", "fccc407923e92b18e2d00b71685b5e7721ea5fe5264bbc951269c7452a672bcc"),
        ("macos", "x86_64", false) => artifact!("macos_x64", "0eced505910fe7c48a2d1e4690d1c9616204ad015c13f8a3cb2d8926903216a5"),
        ("linux", "x86_64", true) | ("windows", "x86_64", false) => artifact!("win_x64", "cee8243e7333aaf716b4f950b9043df94d5763cb5ff0d486a82bd5671cbafa98"),
        ("linux", "x86", true) | ("windows", "x86", false) => artifact!("win_x86", "db3649c65243dc78441abc19334016faf4755a5c3fbe9a1a6e1e3142665db925"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
}

fn get_doorstop_url_and_hash(
    uses_proton: bool,
) -> Result<(&'static str, &'static str, &'static str)> {
    macro_rules! doorstop_artifact {
        ($artifact:literal, $suffix:literal, $hash:literal) => {
            (concat!(
                "https://github.com/mpfaff/UnityDoorstop/releases/download/v4.3.0%2Bmanderrow.2/",
                $artifact,
                $suffix
            ), $hash, $suffix)
        };
    }

    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("linux", "x86_64", false) => doorstop_artifact!("libUnityDoorstop", ".so", "f2dd093de77026400ff8a1c79a8b7ab5417836fedaa764fca4cd655694c2b4a8"),
        ("linux", "x86", false) => todo!(),
        ("macos", "x86_64", false) => doorstop_artifact!("libUnityDoorstop", ".dylib", "bf4fe4074093c9be1911a4616ae5b6f8622a5a78b481b0d41372689b7ac01457"),
        ("linux", "x86_64", true) | ("windows", "x86_64", false) => doorstop_artifact!("UnityDoorstop", ".dll", "5052e430a93850d8698121066fcb4205da0d63de4da98ace1f823c2444896270"),
        ("linux", "x86", true) | ("windows", "x86", false) => todo!(),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
}

pub async fn get_bep_in_ex_path(log: &slog::Logger, uses_proton: bool) -> Result<PathBuf> {
    let (url, hash) = get_url_and_hash(uses_proton)?;
    let path = crate::launching::LOADERS_DIR.join(hash);

    install_zip(
        // TODO: communicate via IPC
        None,
        log,
        &Reqwest(reqwest::Client::new()),
        url,
        Some(crate::installing::CacheOptions::by_hash(hash)),
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
    profile_id: Option<Uuid>,
    doorstop_path: Option<PathBuf>,
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

    let profile_path = profile_id.map(profile_path);

    let temp_dir = tempdir()?.into_path();

    command.env(
        "BEPINEX_CONFIGS",
        profile_path
            .as_ref()
            .map(|p| p.join("config"))
            .unwrap_or_else(|| temp_dir.join("configs")),
    );
    command.env(
        "BEPINEX_PLUGINS",
        profile_path
            .as_ref()
            .map(|p| p.join(MODS_FOLDER))
            .unwrap_or_else(|| temp_dir.join("configs")),
    );
    command.env(
        "BEPINEX_PATCHER_PLUGINS",
        profile_path
            .as_ref()
            .map(|p| p.join("patchers"))
            .unwrap_or_else(|| temp_dir.join("configs")),
    );
    command.env("BEPINEX_CACHE", temp_dir.join("cache"));
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

    command.env("DOORSTOP_ENABLED", "1");
    command.env("DOORSTOP_TARGET_ASSEMBLY", target_assembly);
    command.env("DOORSTOP_IGNORE_DISABLED_ENV", "0");
    // specify these only if they have values
    // command.env("DOORSTOP_MONO_DLL_SEARCH_PATH_OVERRIDE", "");
    command.env("DOORSTOP_MONO_DEBUG_ENABLED", "0");
    command.env("DOORSTOP_MONO_DEBUG_ADDRESS", "127.0.0.1:10000");
    command.env("DOORSTOP_MONO_DEBUG_SUSPEND", "0");
    // specify these only if they have values
    // command.env("DOORSTOP_CLR_CORLIB_DIR", "");
    // command.env("DOORSTOP_CLR_RUNTIME_CORECLR_PATH", "");

    let (doorstop_url, doorstop_hash, doorstop_suffix) = get_doorstop_url_and_hash(uses_proton)?;

    if cfg!(windows) || uses_proton {
        if uses_proton {
            // TODO: don't overwrite anything without checking with the user
            //       via a doctor's note.
            ensure_wine_will_load_dll_override(log, steam_metadata.id, "winhttp").await?;
        }

        install_file(
            // TODO: communicate via IPC
            None,
            log,
            &Reqwest(reqwest::Client::new()),
            doorstop_url,
            // suffix is unnecessary here
            Some(crate::installing::CacheOptions::by_hash(doorstop_hash)),
            &resolve_steam_app_install_directory(steam_metadata.id)
                .await?
                .join("winhttp.dll"),
            None,
        )
        .await?;
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
