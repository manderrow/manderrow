use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context as _, Result};
use lexopt::ValueExt;

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

struct DisplayEnv;
impl std::fmt::Display for DisplayEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (key, value) in std::env::vars_os() {
            f.write_str(" ")?;
            if let Some(key) = key.to_str() {
                f.write_str(key)?;
            } else {
                write!(f, "{:?}", key)?;
            }
            write!(f, "={:?}", value)?;
        }
        Ok(())
    }
}

pub enum WrapperMode {
    Ipc,
    Injection,
}

pub fn run(args: lexopt::Parser, mode: WrapperMode) -> Result<()> {
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

    fn inner1(mut args: lexopt::Parser, mode: WrapperMode) -> Result<()> {
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
        let mut c2s_tx = None::<String>;

        while let Some(arg) = manderrow_args.next()? {
            // NOTE: this can break if an unhandled option's value happens to be `--agent-path` or `--c2s-tx`
            match arg {
                lexopt::Arg::Long("agent-path") => {
                    agent_path = Some(manderrow_args.value()?.into());
                }
                lexopt::Arg::Long("c2s-tx") => {
                    c2s_tx = Some(manderrow_args.value()?.parse()?);
                }
                _ => {}
            }
        }

        writeln!(log_file, "--agent-path: {:?}", agent_path).unwrap();
        writeln!(log_file, "--c2s-tx: {:?}", c2s_tx).unwrap();
        writeln!(log_file, "Args: {}", DisplayArgList).unwrap();
        writeln!(log_file, "Env: {}", DisplayEnv).unwrap();

        match mode {
            WrapperMode::Ipc => super::wrap_with_ipc::inner1(log_file, command_name, args, c2s_tx),
            WrapperMode::Injection => {
                super::wrap_with_injection::inner1(log_file, command_name, args, agent_path)
            }
        }
    }

    match inner1(args, mode) {
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
