use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context as _};
use slog::debug;
use tauri_plugin_http::reqwest;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use zip::result::ZipError;
use zip::ZipArchive;

use crate::commands::profiles::read_profile;
use crate::games::GAMES_BY_ID;
use crate::paths::cache_dir;

use super::steam::paths::resolve_steam_app_install_directory;
use super::steam::proton::{ensure_wine_will_load_dll_override, uses_proton};

#[derive(Debug, thiserror::Error)]
enum InstallZipError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    Blake3HexError(#[from] blake3::HexError),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
}

async fn install_zip(
    log: &slog::Logger,
    url: &str,
    hash_str: &str,
    mut target: PathBuf,
) -> Result<PathBuf, InstallZipError> {
    target.push(hash_str);
    match tokio::fs::metadata(&target).await {
        Ok(m) if m.is_dir() => {
            debug!(log, "Zip is already installed to {target:?}");
            return Ok(target);
        }
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Target exists, but is not a directory",
            )
            .into())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    target.pop();

    let path = cache_dir().join(format!("{hash_str}.zip"));
    let hash = blake3::Hash::from_hex(hash_str)?;
    let hash_on_disk = {
        let mut hsr = blake3::Hasher::new();
        match hsr.update_mmap(&path) {
            Ok(_) => Some(hsr.finalize()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        }
    };

    if hash_on_disk.map(|h| h != hash).unwrap_or(true) {
        let mut resp = reqwest::get(url).await?.error_for_status()?;
        let mut wtr = tokio::fs::File::create(&path).await?;
        while let Some(chunk) = resp.chunk().await? {
            wtr.write_all(&chunk).await?;
        }
        debug!(log, "Cached zip at {path:?}");
    } else {
        debug!(log, "Zip is cached at {path:?}");
    }

    let tmp_dir = tempfile::tempdir_in(&target)?;
    tokio::task::block_in_place(|| {
        let mut archive = ZipArchive::new(std::io::BufReader::new(std::fs::File::open(&path)?))?;
        archive.extract(tmp_dir.path())?;
        Ok::<_, ZipError>(())
    })?;
    target.push(hash_str);
    tokio::fs::rename(tmp_dir.into_path(), &target).await?;
    debug!(log, "Installed zip to {target:?}");
    Ok(target)
}

pub trait CommandBuilder {
    fn env(&mut self, key: impl AsRef<str>, value: impl AsRef<OsStr>);

    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>);
}

pub async fn configure_command(
    log: &slog::Logger,
    command: &mut impl CommandBuilder,
    profile_id: Uuid,
) -> anyhow::Result<()> {
    let profile = read_profile(profile_id).await?;
    let steam_id = GAMES_BY_ID
        .get(&*profile.game)
        .context("No such game")?
        .store_platform_metadata
        .iter()
        .find_map(|m| m.steam_or_direct())
        .context("Unsupported store platform")?;

    let uses_proton = uses_proton(log, steam_id).await?;

    let (url, hash) = match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("macos", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_macos_x64_5.4.23.2.zip", "f90cb47010b52e8d2da1fff4b39b4e95f89dc1de9dddca945b685b9bf8e3ef81"),
        ("linux", "x86_64", true) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x64_5.4.23.2.zip", "d11015bf224343bdc429fbf5ac99bd12fffe115bfa5baf0df4ee81759887a116"),
        ("linux", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_linux_x64_5.4.23.2.zip", "d655acbbb18dc5202c1ba5f6b87288372307868cc62843e3a78a25abf7a50ad3"),
        ("linux", "x86", true) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x86_5.4.23.2.zip", "db8b95c4dca085d20ce5fc7447f6cf9b18469a5d983e535ac8ea5ae8eea828f3"),
        ("linux", "x86", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_linux_x86_5.4.23.2.zip", "99ba36a0d36e6a05db035fd1ac17d9e76740b4e230c598512c07622278222c30"),
        ("windows", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x64_5.4.23.2.zip", "d11015bf224343bdc429fbf5ac99bd12fffe115bfa5baf0df4ee81759887a116"),
        ("windows", "x86", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x86_5.4.23.2.zip", "db8b95c4dca085d20ce5fc7447f6cf9b18469a5d983e535ac8ea5ae8eea828f3"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    };
    let bep_in_ex = install_zip(
        log,
        url,
        hash,
        crate::commands::profiles::profile_path(profile_id),
    )
    .await?;

    if cfg!(windows) || uses_proton {
        command.args(["--doorstop-enable", "true"]);

        command.args(["--doorstop-target-assembly"]);
        if uses_proton {
            let mut buf = OsString::from("Z:");
            buf.push(
                bep_in_ex
                    .as_os_str()
                    .to_str()
                    .context(anyhow!("Invalid Unicode string: {bep_in_ex:?}"))?,
            );
            buf.push("/BepInEx/core/BepInEx.Preloader.dll");
            command.args([buf]);
        } else {
            let mut p = bep_in_ex.clone();
            p.push("BepInEx");
            p.push("core");
            p.push("BepInEx.Preloader.dll");
            command.args([p]);
        }

        command.args(["--doorstop-mono-debug-enabled", "false"]);
        command.args(["--doorstop-mono-debug-address", "127.0.0.1:10000"]);
        command.args(["--doorstop-mono-debug-suspend", "false"]);
        // specify these only if they have values
        // especially --doorstop-mono-dll-search-path-override, which will
        // cause the doorstop to fail if given an empty string
        // command.args(["--doorstop-mono-dll-search-path-override", ""]);
        // command.args(["--doorstop-clr-corlib-dir", ""]);
        // command.args(["--doorstop-clr-runtime-coreclr-path", ""]);
    } else {
        command.env("DOORSTOP_ENABLED", "1");
        command.env(
            "DOORSTOP_TARGET_ASSEMBLY",
            bep_in_ex.join("BepInEx/core/BepInEx.Preloader.dll"),
        );
        command.env("DOORSTOP_IGNORE_DISABLED_ENV", "0");
        command.env("DOORSTOP_MONO_DLL_SEARCH_PATH_OVERRIDE", "");
        command.env("DOORSTOP_MONO_DEBUG_ENABLED", "0");
        command.env("DOORSTOP_MONO_DEBUG_ADDRESS", "127.0.0.1:10000");
        command.env("DOORSTOP_MONO_DEBUG_SUSPEND", "0");
        command.env("DOORSTOP_CLR_CORLIB_DIR", "");
        command.env("DOORSTOP_CLR_RUNTIME_CORECLR_PATH", "");
    }

    if cfg!(windows) || uses_proton {
        if uses_proton {
            // TODO: don't overwrite anything without checking with the user
            //       via a doctor's note.
            ensure_wine_will_load_dll_override(log, steam_id, "winhttp").await?;
        }
        tokio::fs::copy(
            bep_in_ex.join("winhttp.dll"),
            resolve_steam_app_install_directory(steam_id)
                .await?
                .join("winhttp.dll"),
        )
        .await?;
    } else {
        for var in ["LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"] {
            let base = std::env::var_os(var).unwrap_or_else(OsString::new);
            let mut buf = bep_in_ex.as_os_str().to_owned();
            if !base.is_empty() {
                buf.push(":");
                buf.push(base);
            }

            command.env(var, buf);
        }

        for var in ["LD_PRELOAD", "DYLD_INSERT_LIBRARIES"] {
            let base = std::env::var_os(var).unwrap_or_else(OsString::new);
            let mut buf = OsString::from(if cfg!(target_os = "macos") {
                "libdoorstop.dylib"
            } else {
                "libdoorstop.so"
            });
            if !base.is_empty() {
                buf.push(":");
                buf.push(base);
            }

            command.env(var, buf);
        }
    }

    Ok(())
}
