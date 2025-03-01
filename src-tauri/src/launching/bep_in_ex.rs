use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context as _, Result};
use tempfile::tempdir;
use uuid::Uuid;

use crate::commands::profiles::{profile_path, read_profile, MODS_FOLDER};
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
    macro_rules! artifact_url {
        ($target:literal) => {
            concat!(
                "https://github.com/mpfaff/BepInEx/releases/download/v5.4.23.2%2Bbuild.14/BepInEx_",
                $target,
                "_5.4.23.2.zip"
            )
        };
    }

    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("linux", "x86_64", false) => (artifact_url!("linux_x64"), "337947f8889e57336fc8946832c30ca6eced854e3b2b18f454ca5624d074acf9"),
        ("linux", "x86", false) => (artifact_url!("linux_x86"), "44d4b3f91242a778af90d11bdadca0227ce5273164cbcf29c252f7efc087483b"),
        ("macos", "x86_64", false) => (artifact_url!("macos_x64"), "93710bcc2fa45a41bf2d58f8b2eebfd3f4efe5a7e69f6f535985e757c9ddaa19"),
        ("linux", "x86_64", true) | ("windows", "x86_64", false) => (artifact_url!("win_x64"), "fcc1da41089e579268e5ca4a1fc603766eea292a55d5ea1571ff7952af870af8"),
        ("linux", "x86", true) | ("windows", "x86", false) => (artifact_url!("win_x86"), "4061496bcfb593052ffb89224b055a6b3f52a97d1cc1e29cea68bc653b018680"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
}

pub async fn get_bep_in_ex_path(log: &slog::Logger, uses_proton: bool) -> Result<PathBuf> {
    let (url, hash) = get_url_and_hash(uses_proton)?;
    let path = crate::launching::LOADERS_DIR.join(hash);

    install_zip(
        log,
        &Reqwest(reqwest::Client::new()),
        url,
        Some(hash),
        &path,
    )
    .await?
    .finish(log)
    .await?;

    Ok(path)
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

    let bep_in_ex = get_bep_in_ex_path(log, uses_proton).await?;

    let profile_path = profile_path(profile_id);

    command.env("BEPINEX_CONFIGS", profile_path.join("config"));
    command.env("BEPINEX_PLUGINS", profile_path.join(MODS_FOLDER));
    command.env("BEPINEX_PATCHER_PLUGINS", profile_path.join("patchers"));
    command.env("BEPINEX_CACHE", tempdir()?.into_path());

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
