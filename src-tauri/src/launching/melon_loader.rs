use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{bail, Context as _, Result};
use slog::debug;
use uuid::Uuid;

use crate::games::games_by_id;
use crate::installing::install_zip;
use crate::profiles::profile_path;
use crate::stores::steam::paths::resolve_steam_app_install_directory;
use crate::stores::steam::proton::{self, ensure_wine_will_load_dll_override, uses_proton};
use crate::util::process::CommandBuilder;
use crate::Reqwest;

fn get_url_and_hash(uses_proton: bool) -> Result<(&'static str, &'static str)> {
    macro_rules! artifact {
        ($target:literal, $hash:literal) => {
            (concat!(
                "https://github.com/LavaGang/MelonLoader/releases/download/v0.6.6/MelonLoader.",
                $target,
                ".zip"
            ), $hash)
        };
    }

    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("linux", "x86_64", false) => artifact!("Linux.x64", "3ffb5adf7c639f5ffc812117cc9aaeed85b514daacbfd6602e9e04fa3c7f78cc"),
        ("linux", "x86_64", true) | ("windows", "x86_64", false) => artifact!("x64", "72ed91e0c689b1bc32963b6617e6010fe4ba1a369835e3c4b1e99b2a46d2e386"),
        ("linux", "x86", true) | ("windows", "x86", false) => artifact!("x86", "ec0033ba2f04cec4fe1a5bd5804d37e3b111234fb26c916958653e7e0aba320e"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
}

/// Returns the absolute path to the MelonLoader installation. If MelonLoader has not
/// yet been installed, this function will take care of that before returning.
async fn get_path(log: &slog::Logger, uses_proton: bool, profile_id: Uuid) -> Result<PathBuf> {
    let (url, hash) = get_url_and_hash(uses_proton)?;
    let mut path = profile_path(profile_id);
    path.push("MelonLoader");

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
    profile_id: Uuid,
) -> anyhow::Result<()> {
    let steam_metadata = games_by_id()?
        .get(game)
        .context("No such game")?
        .store_platform_metadata
        .iter()
        .find_map(|m| m.steam_or_direct())
        .context("Unsupported store platform")?;

    let uses_proton = uses_proton(log, steam_metadata.id).await?;

    let loader_path = get_path(log, uses_proton, profile_id).await?;

    command.arg("--melonloader.basedir");
    command.arg(if uses_proton {
        let mut buf = proton::linux_root().to_owned();
        buf.reserve_exact(loader_path.as_os_str().len());
        buf.push(loader_path.as_os_str());
        PathBuf::from(buf)
    } else {
        loader_path.clone()
    });

    let proxy_path = loader_path.join("version.dll");

    if cfg!(windows) || uses_proton {
        if uses_proton {
            // TODO: don't overwrite anything without checking with the user
            //       via a doctor's note.
            ensure_wine_will_load_dll_override(log, steam_metadata.id, "winhttp").await?;
        }

        let proxy_install_target = resolve_steam_app_install_directory(steam_metadata.id)
            .await?
            .join("winhttp.dll");

        tokio::fs::copy(proxy_path, &proxy_install_target).await?;
    } else {
        const VAR: &str = if cfg!(target_os = "macos") {
            "DYLD_INSERT_LIBRARIES"
        } else {
            "LD_PRELOAD"
        };
        let base = std::env::var_os(VAR).unwrap_or_else(OsString::new);
        let mut buf = proxy_path.into_os_string();
        if !base.is_empty() {
            buf.push(":");
            buf.push(base);
        }

        debug!(log, "Injecting {VAR} {buf:?}");

        command.env(VAR, buf);
    }

    Ok(())
}
