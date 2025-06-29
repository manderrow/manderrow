use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

pub fn inner1(
    mut log_file: std::fs::File,
    command_name: OsString,
    args: Vec<OsString>,
    agent_path: Option<PathBuf>,
) -> Result<()> {
    let mut command = Command::new(&command_name);
    command.args(args);

    if let Some(agent_path) = agent_path {
        if cfg!(unix) {
            const VAR: &str = if cfg!(target_os = "macos") {
                "DYLD_INSERT_LIBRARIES"
            } else {
                "LD_PRELOAD"
            };
            let base = std::env::var_os(VAR).unwrap_or_else(OsString::new);
            let mut buf = agent_path.into_os_string();
            if !base.is_empty() {
                buf.push(":");
                buf.push(base);
            }

            writeln!(log_file, "Injecting {VAR} {buf:?}").unwrap();

            command.env(VAR, buf);
        }
    }

    let mut child = match command.spawn() {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(
                anyhow::Error::new(e).context(format!("Could not locate command {command_name:?}"))
            )
        }
        Err(e) => return Err(e.into()),
    };

    let status = child.wait()?;

    status.exit_ok()?;

    Ok(())
}
