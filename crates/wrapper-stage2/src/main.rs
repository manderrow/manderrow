use std::{collections::HashMap, ffi::OsString};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let env =
        serde_json::from_str::<HashMap<String, OsString>>(&std::env::var("MANDERROW_WRAPPER_ENV")?)?;
    let mut args = std::env::args_os();
    _ = args.next();
    let command = args.next().context("Missing required argument COMMAND")?;
    let status = std::process::Command::new(command)
        .args(args)
        .envs(env)
        .status()?;
    std::process::exit(
        status
            .code()
            .context("Child process exitted without a status code")?,
    )
}
