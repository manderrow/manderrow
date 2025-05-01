#![deny(unused_must_use)]
#![feature(int_from_ascii)]
#![feature(slice_split_once)]

pub mod wait_group;

use std::num::NonZeroU32;

use anyhow::Result;
use slog::Logger;

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Pid(NonZeroU32);

#[derive(Debug, thiserror::Error)]
pub enum WaitForExitError {}

impl Pid {
    pub fn from_raw(value: NonZeroU32) -> Self {
        Self(value)
    }

    #[cfg(unix)]
    fn rustix_pid(self) -> rustix::process::Pid {
        rustix::process::Pid::from_raw(self.0.cast_signed().get())
            .expect("non-zero in, non-zero out")
    }

    pub async fn wait_for_exit(self, log: &Logger) -> Result<()> {
        #[cfg(windows)]
        {
            use anyhow::bail;
            use winsafe::prelude::*;

            let pid = self.0;

            let proc = winsafe::HPROCESS::OpenProcess(
                winsafe::co::PROCESS::SYNCHRONIZE,
                false,
                pid.get(),
            )?;

            slog::info!(log, "Waiting for process {pid:?} to shut down");

            // TODO: detect "not found" and return correct result
            tokio::task::spawn_blocking(move || {
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

            let pid = self.rustix_pid();

            slog::info!(log, "Waiting for process {pid:?} to shut down");
            // TODO: use https://man.freebsd.org/cgi/man.cgi?query=kvm_getprocs instead of spawning
            // a process every time
            while tokio::process::Command::new("ps")
                .args([
                    "-p",
                    itoa::Buffer::new().format(pid.as_raw_nonzero().get() as u32),
                ])
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

            let pid = self.rustix_pid();

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

            tokio::task::spawn_blocking(move || {
                let mut pollfd = rustix::event::PollFd::new(&pidfd, rustix::event::PollFlags::IN);
                loop {
                    rustix::event::poll(std::slice::from_mut(&mut pollfd), None)?;
                    if pollfd.revents().contains(rustix::event::PollFlags::IN) {
                        break;
                    }
                }
                Ok(())
            })
            .await?
        }
    }

    pub fn kill(self, log: &Logger, hard: bool) -> Result<()> {
        #[cfg(windows)]
        {
            use winsafe::prelude::*;

            // not supported
            _ = hard;

            let pid = self.0;

            let proc =
                winsafe::HPROCESS::OpenProcess(winsafe::co::PROCESS::TERMINATE, false, pid.get())?;

            slog::info!(log, "Killing process {pid:?}");

            proc.TerminateProcess(1)?;
        }
        #[cfg(unix)]
        {
            let pid = self.rustix_pid();

            slog::info!(log, "Killing process {pid:?}");

            rustix::process::kill_process(
                pid,
                if hard {
                    rustix::process::Signal::KILL
                } else {
                    rustix::process::Signal::TERM
                },
            )?;
        }
        Ok(())
    }
}
