use anyhow::Context;

/// Starts a new instance of the app that waits for this one to exit. The caller is responsible for
/// quitting this instance afterwards.
pub fn relaunch() -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("Failed to locate the executable")?;
    std::process::Command::new(exe)
        .arg("--relaunch")
        .arg(std::process::id().to_string())
        .spawn()
        .context("Failed to start a new instance")?;
    Ok(())
}
