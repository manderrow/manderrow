use std::path::PathBuf;

use anyhow::{bail, Result};
use manderrow_types::games::Game;
use tauri::AppHandle;
use uuid::Uuid;

use crate::installing::install_zip;
use crate::profiles::profile_path;
use crate::stores::steam::proton;
use crate::Reqwest;

use super::InstructionEmitter;

struct Artifact {
    url: &'static str,
    hash: &'static str,
    lib_file_name: &'static str,
}

fn get_url_and_hash(uses_proton: bool) -> Result<Artifact> {
    macro_rules! artifact {
        ($target:literal, $hash:literal, $lib_file_name:literal) => {
            Artifact {
                url: concat!(
                    "https://github.com/LavaGang/MelonLoader/releases/download/v0.6.6/MelonLoader.",
                    $target,
                    ".zip"
                ),
                hash: $hash,
                lib_file_name: $lib_file_name,
            }
        };
    }

    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("linux", "x86_64", false) => artifact!("Linux.x64", "3ffb5adf7c639f5ffc812117cc9aaeed85b514daacbfd6602e9e04fa3c7f78cc", "libversion.so"),
        ("linux", "x86_64", true) | ("windows", "x86_64", false) => artifact!("x64", "72ed91e0c689b1bc32963b6617e6010fe4ba1a369835e3c4b1e99b2a46d2e386", "version.dll"),
        ("linux", "x86", true) | ("windows", "x86", false) => artifact!("x86", "ec0033ba2f04cec4fe1a5bd5804d37e3b111234fb26c916958653e7e0aba320e", "version.dll"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
}

/// Returns the absolute path to the MelonLoader installation. If MelonLoader has not
/// yet been installed, this function will take care of that before returning.
async fn get_path(log: &slog::Logger, uses_proton: bool, profile_id: Uuid) -> Result<(PathBuf, &'static str)> {
    let Artifact { url, hash, lib_file_name } = get_url_and_hash(uses_proton)?;
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

    Ok((path, lib_file_name))
}

pub async fn emit_instructions(
    app: Option<&AppHandle>,
    log: &slog::Logger,
    mut em: InstructionEmitter<'_>,
    game: &Game<'_>,
    profile_id: Uuid,
    uses_proton: bool,
) -> anyhow::Result<()> {
    let (loader_path, lib_file_name) = get_path(log, uses_proton, profile_id).await?;

    em.append_arg("--melonloader.basedir");
    em.append_arg(if uses_proton {
        let mut buf = proton::linux_root().to_owned();
        buf.reserve_exact(loader_path.as_os_str().len());
        buf.push(loader_path.as_os_str());
        PathBuf::from(buf)
    } else {
        loader_path.clone()
    });

    em.load_library(loader_path.join(lib_file_name));

    Ok(())
}
