use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context as _, Result};

struct DisplayArgList;
impl std::fmt::Display for DisplayArgList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = std::env::args_os();
        if let Some(arg) = iter.next() {
            write!(f, "{:?}", arg)?;
            for arg in iter {
                write!(f, " {:?}", arg)?;
            }
        }
        Ok(())
    }
}

pub fn run(args: lexopt::Parser) -> Result<()> {
    std::panic::set_backtrace_style(std::panic::BacktraceStyle::Full);
    std::panic::set_hook(Box::new(|info| {
        _ = std::fs::write(
            "manderrow-wrap-crash.txt",
            format!(
                "{}\nargs: {}",
                if let Some(&s) = info.payload().downcast_ref::<&'static str>() {
                    s
                } else if let Some(s) = info.payload().downcast_ref::<String>() {
                    s.as_str()
                } else {
                    "Box<dyn Any>"
                },
                DisplayArgList
            ),
        );
    }));

    std::fs::write("manderrow-wrap-args.txt", DisplayArgList.to_string()).unwrap();

    fn inner1(mut args: lexopt::Parser) -> Result<()> {
        use lexopt::Arg::*;

        let command_name = match args.next()?.context("Missing required argument BINARY")? {
            Value(s) => s,
            arg => return Err(arg.unexpected().into()),
        };

        let args = args.raw_args()?.collect::<Vec<_>>();

        // TODO: avoid cloning so much. Not just here. All over dealing with arguments.
        let (manderrow_args, _) = manderrow_args::extract(args.iter().cloned())?;

        let mut log_file = std::fs::File::create("manderrow-wrap.log").unwrap();

        let mut manderrow_args = lexopt::Parser::from_args(manderrow_args);

        let mut agent_path = None::<PathBuf>;

        while let Some(arg) = manderrow_args.next()? {
            match arg {
                lexopt::Arg::Long("agent-path") => {
                    agent_path = Some(manderrow_args.value()?.into());
                }
                _ => {}
            }
        }

        writeln!(log_file, "Agent path: {:?}", agent_path).unwrap();
        writeln!(
            log_file,
            "Args: {:?}",
            std::env::args_os().collect::<Vec<_>>()
        )
        .unwrap();
        writeln!(
            log_file,
            "Env: {:?}",
            std::env::vars_os().collect::<Vec<_>>()
        )
        .unwrap();

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
                return Err(anyhow::Error::new(e)
                    .context(format!("Could not locate command {command_name:?}")))
            }
            Err(e) => return Err(e.into()),
        };

        let status = child.wait()?;

        status.exit_ok()?;

        Ok(())
    }

    match inner1(args) {
        Ok(()) => Ok(()),
        Err(e) => {
            std::fs::write(
                "manderrow-wrap-crash.txt",
                format!("{e}\nargs: {}", DisplayArgList),
            )
            .unwrap();
            Err(e)
        }
    }
}
