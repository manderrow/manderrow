use anyhow::Result;
use slog::Logger;

#[derive(Clone, Copy)]
pub struct Pid {
    #[cfg(windows)]
    pub value: u32,
    #[cfg(unix)]
    pub value: rustix::process::Pid,
}

impl Pid {
    pub async fn wait_for_exit(self, log: &Logger) -> Result<()> {
        let pid = self.value;
        #[cfg(windows)]
        {
            use anyhow::bail;
            use winsafe::prelude::*;

            // TODO: detect "not found" and return correct result
            tokio::task::spawn_blocking(|| {
                let proc =
                    winsafe::HPROCESS::OpenProcess(winsafe::co::PROCESS::SYNCHRONIZE, false, pid)?;
                let event = proc.WaitForSingleObject(None)?;
                if event != winsafe::co::WAIT::OBJECT_0 {
                    bail!("Unexpected WAIT_EVENT: {event:?}");
                }
                Ok(())
            })
            .await?
        }
        #[cfg(target_os = "macos")]
        {
            use std::process::Stdio;
            use std::time::Duration;

            slog::info!(log, "Waiting for process {pid:?} to shut down");
            // TODO: use https://man.freebsd.org/cgi/man.cgi?query=kvm_getprocs instead of spawning
            // a process every time
            while tokio::process::Command::new("ps")
                .args(["-p", itoa::Buffer::new().format(pid.as_raw_nonzero().get())])
                .stdout(Stdio::null())
                .status()
                .await?
                .success()
            {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Ok(())
        }
        #[cfg(target_os = "linux")]
        {
            use anyhow::bail;
            use slog::info;

            tokio::task::spawn_blocking(move || {
                slog_scope::with_logger(move |log| {
                    let pidfd = match rustix::process::pidfd_open(
                        pid,
                        rustix::process::PidfdFlags::empty(),
                    ) {
                        Ok(t) => t,
                        Err(rustix::io::Errno::SRCH) => {
                            info!(log, "Process {pid:?} has already shut down");
                            return Ok(());
                        }
                        Err(errno) => bail!("pidfd_open errno={errno}"),
                    };

                    info!(log, "Waiting for process {pid:?} to shut down");

                    let mut pollfd =
                        rustix::event::PollFd::new(&pidfd, rustix::event::PollFlags::IN);
                    loop {
                        rustix::event::poll(std::slice::from_mut(&mut pollfd), None)?;
                        if pollfd.revents().contains(rustix::event::PollFlags::IN) {
                            break;
                        }
                    }
                    Ok(())
                })
            })
            .await?
        }
    }
}
