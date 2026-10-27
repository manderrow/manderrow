#![deny(unused_must_use)]
#![feature(error_generic_member_access)]
#![feature(exit_status_error)]
#![feature(extend_one)]
#![feature(map_try_insert)]
#![feature(os_string_truncate)]
#![feature(panic_backtrace_config)]
#![feature(type_alias_impl_trait)]
#![feature(type_changing_struct_update)]
#![feature(vec_push_within_capacity)]

mod app;
mod error;
mod games;
mod i18n;
mod importing;
mod installing;
mod ipc;
mod launching;
mod markdown;
mod mod_index;
mod profiles;
mod qt;
mod settings;
mod single_instance;
mod stores;
mod tasks;
mod util;
mod window_state;
#[cfg(windows)]
mod windows_console;
mod wrap;
mod wrap_with_injection;

use std::num::NonZeroU32;
use std::ops::Deref;

use anyhow::{bail, Context};
use ipc::IpcState;

pub use error::{CommandError, Error};
use lexopt::ValueExt;

#[derive(Clone)]
struct Reqwest(reqwest::Client);

/// The user agent of the app's requests, which some servers require one of, as Wikimedia's do for
/// the images that READMEs show from them.
const USER_AGENT: &str = concat!(
    "Manderrow/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/manderrow/manderrow)"
);

impl Reqwest {
    /// A client for the app's requests.
    fn new() -> reqwest::Result<Self> {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map(Self)
    }
}

impl Deref for Reqwest {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn run_app() -> anyhow::Result<()> {
    let Some(instance) = single_instance::acquire()? else {
        slog_scope::info!("Manderrow is already running");
        return Ok(());
    };

    // Backend state is created inside the runtime, since parts of it spawn tasks.
    let _runtime = qt::runtime().enter();

    let app = app::AppHandle::new(Reqwest::new()?, settings::try_read());
    app.set_ipc_state(IpcState::new(app.clone(), slog_scope::logger()));

    qt::run(app, instance)
}

pub fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    windows_console::attach();

    manderrow_paths::init().unwrap();

    // Qt reads its own arguments, of which QML tools pass this one in debug builds
    let mut args = lexopt::Parser::from_args(std::env::args_os().skip(1).filter(|arg| {
        !(cfg!(debug_assertions) && arg.to_string_lossy().starts_with("-qmljsdebugger="))
    }));

    let mut relaunch = None::<u32>;

    use lexopt::Arg::*;
    while let Some(arg) = args.next()? {
        match arg {
            Value(cmd) if cmd == "wrap-with-injection" => {
                return wrap::run(args, wrap::WrapperMode::Injection)
            }
            Value(cmd) => bail!("Unrecognized command {cmd:?}"),
            Long("relaunch") => relaunch = Some(args.value()?.parse()?),
            arg => return Err(arg.unexpected().into()),
        }
    }

    #[cfg(windows)]
    windows_console::show_main_thread_panics();

    let _guard = slog_envlogger::init()?;
    slog_scope::info!(
        "Data is in {:?}, settings in {:?}, and the cache in {:?}",
        manderrow_paths::local_data_dir(),
        manderrow_paths::config_dir(),
        manderrow_paths::cache_dir()
    );

    if let Some(pid) = relaunch {
        slog_scope::with_logger(|log| {
            let pid = NonZeroU32::new(pid).context("null pid")?;
            // ignore errors because the process might die before or during this operation
            _ = manderrow_process_util::Pid::from_raw(pid).wait_for_exit(log);
            Ok::<_, anyhow::Error>(())
        })?;
    }

    let result = run_app();
    #[cfg(windows)]
    if let Err(e) = &result {
        windows_console::show_fatal_error(&format!("{e:?}"));
    }
    result
}
