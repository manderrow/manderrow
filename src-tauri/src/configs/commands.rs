use std::path::PathBuf;

use anyhow::Context;
use tauri::{ipc::Response, AppHandle, Emitter};
use uuid::Uuid;

use crate::{configs::ConfigOptions, CommandError};

use super::{File, Patch};

#[tauri::command]
pub async fn scan_mod_configs(profile: Uuid) -> Result<Response, CommandError> {
    let (root, paths) = super::scan_configs(profile).await?;
    let mut buf = Vec::new();
    buf.push(b'[');
    for (i, path) in paths.iter().enumerate() {
        if i != 0 {
            buf.push(b',');
        }
        serde_json::to_writer(&mut buf, path.strip_prefix(&root).unwrap())
            .context("Failed to encode path to JSON")?;
    }
    buf.push(b']');
    Ok(Response::new(String::from_utf8(buf).unwrap()))
}

#[tauri::command]
pub async fn read_mod_config(
    profile: Uuid,
    path: PathBuf,
    options: ConfigOptions,
) -> Result<File, CommandError> {
    Ok(super::read_config(profile, &path, options).await?)
}

#[tauri::command]
pub async fn update_mod_config(
    profile: Uuid,
    path: PathBuf,
    options: ConfigOptions,
    patches: Vec<Patch>,
) -> Result<File, CommandError> {
    Ok(super::update_config(profile, &path, options, &patches).await?)
}
