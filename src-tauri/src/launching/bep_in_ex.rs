use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context as _, Result};
use uuid::Uuid;

use crate::commands::profiles::read_profile;
use crate::games::GAMES_BY_ID;
use crate::installing::install_zip;
use crate::Reqwest;

use super::steam::paths::resolve_steam_app_install_directory;
use super::steam::proton::{ensure_wine_will_load_dll_override, uses_proton};

pub trait CommandBuilder {
    fn env(&mut self, key: impl AsRef<str>, value: impl AsRef<OsStr>);

    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>);
}

fn get_url_and_hash(uses_proton: bool) -> Result<(&'static str, &'static str)> {
    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("macos", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_macos_x64_5.4.23.2.zip", "f90cb47010b52e8d2da1fff4b39b4e95f89dc1de9dddca945b685b9bf8e3ef81"),
        ("linux", "x86_64", true) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x64_5.4.23.2.zip", "d11015bf224343bdc429fbf5ac99bd12fffe115bfa5baf0df4ee81759887a116"),
        ("linux", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_linux_x64_5.4.23.2.zip", "d655acbbb18dc5202c1ba5f6b87288372307868cc62843e3a78a25abf7a50ad3"),
        ("linux", "x86", true) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x86_5.4.23.2.zip", "db8b95c4dca085d20ce5fc7447f6cf9b18469a5d983e535ac8ea5ae8eea828f3"),
        ("linux", "x86", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_linux_x86_5.4.23.2.zip", "99ba36a0d36e6a05db035fd1ac17d9e76740b4e230c598512c07622278222c30"),
        ("windows", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x64_5.4.23.2.zip", "d11015bf224343bdc429fbf5ac99bd12fffe115bfa5baf0df4ee81759887a116"),
        ("windows", "x86", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x86_5.4.23.2.zip", "db8b95c4dca085d20ce5fc7447f6cf9b18469a5d983e535ac8ea5ae8eea828f3"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
}

pub const BEP_IN_EX_FOLDER: &str = "BepInEx";

pub fn get_bep_in_ex_path(profile_id: Uuid) -> PathBuf {
    let mut p = crate::commands::profiles::profile_path(profile_id);
    p.push(BEP_IN_EX_FOLDER);
    p
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

    let (url, hash) = get_url_and_hash(uses_proton)?;
    let bep_in_ex = get_bep_in_ex_path(profile_id);
    install_zip(
        log,
        &Reqwest(reqwest::Client::new()),
        url,
        Some(hash),
        &bep_in_ex,
    )
    .await?
    .finish(log)
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
