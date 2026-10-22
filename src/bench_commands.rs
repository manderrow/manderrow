use crate::CommandError;

pub async fn bench_exit_splash() -> Result<(), CommandError> {
    if std::env::var_os("EXIT_ON_SPLASH").is_some() {
        std::process::exit(0);
    }
    Ok(())
}

pub async fn bench_exit_interactive() -> Result<(), CommandError> {
    if std::env::var_os("EXIT_ON_INTERACTIVE").is_some() {
        std::process::exit(0);
    }
    Ok(())
}
