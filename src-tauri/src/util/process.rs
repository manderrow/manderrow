use anyhow::{bail, Result};
use slog::{info, Logger};

#[derive(Clone, Copy)]
pub struct Pid {
    #[cfg(windows)]
    pub value: u32,
    #[cfg(unix)]
    pub value: rustix::process::Pid,
}

impl Pid {
    pub fn wait_for_exit(self, log: &Logger) -> Result<()> {
        let pid = self.value;
        #[cfg(windows)]
        {
            use winsafe::prelude::*;
            // TODO: detect "not found" and return correct result
            let proc =
                winsafe::HPROCESS::OpenProcess(winsafe::co::PROCESS::SYNCHRONIZE, false, pid)?;
            let event = proc.WaitForSingleObject(None)?;
            if event != winsafe::co::WAIT::OBJECT_0 {
                bail!("Unexpected WAIT_EVENT: {event:?}");
            }
            Ok(())
        }
        #[cfg(unix)]
        {
            let pidfd = match rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty())
            {
                Ok(t) => t,
                Err(rustix::io::Errno::SRCH) => {
                    info!(log, "Process {pid:?} has already shut down");
                    return Ok(());
                }
                Err(errno) => bail!("pidfd_open errno={errno}"),
            };

            info!(log, "Waiting for process {pid:?} to shut down");

            let mut pollfd = rustix::event::PollFd::new(&pidfd, rustix::event::PollFlags::IN);
            loop {
                rustix::event::poll(std::slice::from_mut(&mut pollfd), None)?;
                if pollfd.revents().contains(rustix::event::PollFlags::IN) {
                    break;
                }
            }
            Ok(())
        }
    }
}
