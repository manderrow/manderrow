//! Crash reporting helpers.

use std::backtrace::Backtrace;
use std::panic::AssertUnwindSafe;

use manderrow_ipc::C2SMessage;

use crate::init::ipc;

#[no_panic::no_panic]
fn forget_on_panic(f: impl FnOnce() -> () + std::panic::UnwindSafe) {
    std::mem::forget(std::panic::catch_unwind(f))
}

#[no_panic::no_panic]
#[track_caller]
pub fn report_crash(error: impl std::fmt::Display) {
    let report = CrashReport::new(error);
    forget_on_panic(AssertUnwindSafe(|| {
        if let Ok(f) = std::fs::File::create("manderrow-agent-crash.txt") {
            _ = report.dump(f);
        }
    }));
    forget_on_panic(AssertUnwindSafe(|| {
        _ = report.dump(std::io::stderr().lock());
    }));
    forget_on_panic(AssertUnwindSafe(|| {
        if let Some(ipc) = ipc() {
            _ = ipc.send(C2SMessage::Crash {
                error: report.err.to_string(),
            });
        }
    }));
    // drop can panic, forget it.
    std::mem::forget(report);
}

struct CrashReport<E> {
    err: E,
    backtrace: std::thread::Result<Backtrace>,
}

impl<E> CrashReport<E> {
    fn new(err: E) -> Self {
        Self {
            err,
            // under absolutely no circumstances will we panic during the crash report process
            backtrace: std::panic::catch_unwind(Backtrace::force_capture),
        }
    }

    fn dump(&self, mut w: impl std::io::Write) -> std::io::Result<()>
    where
        E: std::fmt::Display,
    {
        std::panic::catch_unwind(AssertUnwindSafe(move || {
            write!(w, "{}", self.err)?;
            dump_crash_report_common(&self.backtrace, w)
        }))
        .unwrap_or(Ok(()))
    }
}

fn dump_crash_report_common(
    backtrace: &std::thread::Result<Backtrace>,
    mut w: impl std::io::Write,
) -> std::io::Result<()> {
    write!(w, "\n\nBacktrace:\n")?;
    match backtrace {
        Ok(bt) => writeln!(w, "{}", bt)?,
        Err(e) => writeln!(w, "{:?}", e)?,
    }
    write!(w, "{}", DumpEnvironment)
}

pub struct DumpEnvironment;

impl std::fmt::Display for DumpEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Env: {{")?;
        match std::panic::catch_unwind(AssertUnwindSafe(|| {
            for (key, value) in std::env::vars_os() {
                writeln!(f, "  {:?}={:?}", key, value)?;
            }
            Ok(())
        })) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => writeln!(f, "\n  Caught panic while dumping environment: {:?}", e)?,
        }
        write!(f, "}}\n\nArgs:")?;
        match std::panic::catch_unwind(AssertUnwindSafe(|| {
            for arg in std::env::args_os() {
                write!(f, " {:?}", arg)?;
            }
            Ok(())
        })) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => writeln!(f, "\n  Caught panic while dumping arguments: {:?}", e)?,
        }
        write!(f, "\n\nCwd:")?;
        match std::panic::catch_unwind(AssertUnwindSafe(|| match std::env::current_dir() {
            Ok(path) => write!(f, " {:?}", path),
            Err(e) => write!(f, "\n  Error: {}", e),
        })) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => writeln!(
                f,
                "\n  Caught panic while dumping working directory: {:?}",
                e
            )?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_no_panic() {
        use core::hint::black_box;
        if black_box(false) {
            super::report_crash("");
        }
    }
}
