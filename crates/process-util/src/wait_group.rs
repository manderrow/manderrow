use crate::Pid;

pub fn wait_group<T>() -> (Submitter<T>, Waiter<T>) {
    let (submitter, waiter) = sys::wait_group();
    (Submitter(submitter), Waiter(waiter))
}

#[derive(Clone)]
pub struct Submitter<T>(sys::Submitter<T>);

pub struct Waiter<T>(sys::Waiter<T>);

#[derive(Debug, thiserror::Error)]
pub enum SubmitError {
    #[error("Closed")]
    Closed,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum WaitError {
    #[error("Closed")]
    Closed,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl<T> Submitter<T> {
    pub fn submit(&self, pid: Pid, data: T) -> Result<(), SubmitError> {
        self.0.submit(pid, data)
    }
}

impl<T> Waiter<T> {
    pub fn wait_for_any(&mut self, log: &slog::Logger) -> Result<T, WaitError> {
        self.0.wait_for_any(log)
    }
}

#[cfg(windows)]
mod sys {
    use std::hash::Hash;
    use std::mem::{ManuallyDrop, MaybeUninit};
    use std::ops::ControlFlow;
    use std::ptr::NonNull;
    use std::sync::mpsc::{Receiver, Sender, channel};

    use anyhow::{Context, anyhow};
    use slog::warn;
    use winsafe::guard::CloseHandleGuard;
    use winsafe::prelude::*;
    use winsafe::{HEVENT, HPROCESS};

    use crate::Pid;

    use super::{SubmitError, WaitError};

    pub fn wait_group<T>() -> (Submitter<T>, Waiter<T>) {
        let (tx, rx) = channel();
        let mut attrs = winsafe::SECURITY_ATTRIBUTES::default();
        attrs.set_bInheritHandle(false);
        let mut notification = ManuallyDrop::new(
            HEVENT::CreateEvent(Some(&mut attrs), false, false, None)
                .expect("Failed to create notification event"),
        );
        (
            Submitter {
                tx,
                notification: Notification(dup_handle(&*notification)),
            },
            Waiter {
                handles: vec![unsafe {
                    CloseHandleGuard::new(SendSyncHANDLE::from_unsafe(notification.leak()))
                }],
                data: Vec::new(),
                rx,
            },
        )
    }

    struct Notification(CloseHandleGuard<HEVENT>);

    // SAFETY: https://stackoverflow.com/a/12214212/10082531 says handles are thread-safe unless documented otherwise
    unsafe impl Sync for Notification {}

    pub struct Submitter<T> {
        tx: Sender<(Pid, T)>,
        notification: Notification,
    }

    #[derive(Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    struct SendSyncHANDLE(*mut std::ffi::c_void);

    unsafe impl Send for SendSyncHANDLE {}
    unsafe impl Sync for SendSyncHANDLE {}

    impl SendSyncHANDLE {
        pub fn from<T: Handle + Send + Sync>(handle: T) -> Self {
            unsafe { Self::from_unsafe(handle) }
        }

        pub unsafe fn from_unsafe<T: Handle>(handle: T) -> Self {
            Self(handle.ptr())
        }
    }

    impl std::fmt::Display for SendSyncHANDLE {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:#010x}", self.0 as usize)
        }
    }

    impl std::fmt::LowerHex for SendSyncHANDLE {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::LowerHex::fmt(&(self.0 as usize), f)
        }
    }

    impl std::fmt::UpperHex for SendSyncHANDLE {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::UpperHex::fmt(&(self.0 as usize), f)
        }
    }

    impl Handle for SendSyncHANDLE {
        const NULL: Self = Self(std::ptr::null_mut());
        const INVALID: Self = Self(-1 as _);

        unsafe fn from_ptr(p: *mut std::ffi::c_void) -> Self {
            Self(p)
        }

        unsafe fn as_mut(&mut self) -> &mut *mut std::ffi::c_void {
            &mut self.0
        }

        fn ptr(&self) -> *mut std::ffi::c_void {
            self.0
        }
    }

    pub struct Waiter<T> {
        handles: Vec<CloseHandleGuard<SendSyncHANDLE>>,
        data: Vec<T>,
        rx: Receiver<(Pid, T)>,
    }

    impl<T> Submitter<T> {
        pub fn submit(&self, pid: Pid, data: T) -> Result<(), SubmitError> {
            self.tx.send((pid, data)).map_err(|_| SubmitError::Closed)?;
            self.notification
                .0
                .SetEvent()
                .context("Failed to notify waiter")?;
            Ok(())
        }
    }

    impl<T> Waiter<T> {
        fn register_pid(&mut self, pid: Pid, data: T) -> ControlFlow<T> {
            let Ok(mut proc) = winsafe::HPROCESS::OpenProcess(
                winsafe::co::PROCESS::SYNCHRONIZE,
                false,
                pid.0.get(),
            ) else {
                // TODO: verify that the process is not found vs other errors
                return ControlFlow::Break(data);
            };
            self.handles
                .push(unsafe { CloseHandleGuard::new(SendSyncHANDLE::from_unsafe(proc.leak())) });
            self.data.push(data);
            ControlFlow::Continue(())
        }

        pub fn wait_for_any(&mut self, log: &slog::Logger) -> Result<T, WaitError> {
            if self.handles.is_empty() {
                let (pid, data) = self.rx.recv().map_err(|_| WaitError::Closed)?;
                if let ControlFlow::Break(data) = self.register_pid(pid, data) {
                    return Ok(data);
                }
            }
            while let Ok((pid, data)) = self.rx.try_recv() {
                if let ControlFlow::Break(data) = self.register_pid(pid, data) {
                    return Ok(data);
                }
            }
            let count = std::cmp::min(
                self.handles.len(),
                windows::Win32::System::SystemServices::MAXIMUM_WAIT_OBJECTS as usize,
            );
            if count != self.handles.len() {
                // TODO: iterate over chunks, put a timeout on every chunk, repeat until
                //       we find something
                warn!(
                    log,
                    "Truncating list of process handles (length: {}) due to Windows API limitations",
                    self.handles.len()
                );
            }
            let event = unsafe {
                windows::Win32::System::Threading::WaitForMultipleObjects(
                    NonNull::slice_from_raw_parts(
                        NonNull::from(&self.handles[..count]).cast(),
                        count,
                    )
                    .as_ref(),
                    false,
                    windows::Win32::System::Threading::INFINITE,
                )
            };
            if !(windows::Win32::Foundation::WAIT_OBJECT_0.0
                ..(windows::Win32::Foundation::WAIT_OBJECT_0.0 + count as u32))
                .contains(&event.0)
            {
                return Err(anyhow!("Unexpected WAIT_EVENT: {event:?}").into());
            }
            let i = (event.0 - windows::Win32::Foundation::WAIT_OBJECT_0.0) as usize;
            self.handles.swap_remove(i);
            let data = self.data.swap_remove(i);
            Ok(data)
        }
    }

    impl<T> Clone for Submitter<T> {
        fn clone(&self) -> Self {
            Self {
                tx: self.tx.clone(),
                notification: Notification(dup_handle(&*self.notification.0)),
            }
        }
    }

    fn dup_handle<T: Handle>(handle: &T) -> CloseHandleGuard<T> {
        let process = HPROCESS::GetCurrentProcess();
        let mut new_handle = MaybeUninit::uninit();
        unsafe {
            windows::Win32::Foundation::DuplicateHandle(
                windows::Win32::Foundation::HANDLE(process.ptr()),
                windows::Win32::Foundation::HANDLE(handle.ptr()),
                windows::Win32::Foundation::HANDLE(process.ptr()),
                new_handle.as_mut_ptr(),
                0,
                false,
                windows::Win32::Foundation::DUPLICATE_SAME_ACCESS,
            )
            .expect("Failed to duplicate handle");
        }
        // SAFETY: just called DuplicateHandle and checked for errors
        unsafe { CloseHandleGuard::new(T::from_ptr(new_handle.assume_init().0)) }
    }
}

#[cfg(target_os = "macos")]
mod sys {
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::Once;
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::time::Duration;

    use anyhow::{Context, anyhow};
    use slog::error;

    use crate::Pid;

    use super::{SubmitError, WaitError};

    pub fn wait_group<T>() -> (Submitter<T>, Waiter<T>) {
        let (tx, rx) = channel();
        (
            Submitter { tx },
            Waiter {
                entries: Vec::new(),
                rx,
                seen_buf: Vec::new(),
                p_buf: String::new(),
                stdout_buf: Vec::new(),
            },
        )
    }

    #[derive(Clone)]
    pub struct Submitter<T> {
        tx: Sender<(Pid, T)>,
    }

    pub struct Waiter<T> {
        entries: Vec<(Pid, T)>,
        rx: Receiver<(Pid, T)>,
        // TODO: replace with a bit set
        seen_buf: Vec<bool>,
        p_buf: String,
        stdout_buf: Vec<u8>,
    }

    impl<T> Submitter<T> {
        pub fn submit(&self, pid: Pid, data: T) -> Result<(), SubmitError> {
            self.tx.send((pid, data)).map_err(|_| SubmitError::Closed)
        }
    }

    impl<T> Waiter<T> {
        pub fn wait_for_any(&mut self, log: &slog::Logger) -> Result<T, WaitError> {
            loop {
                let try_more = if self.entries.is_empty() {
                    self.entries
                        .push(self.rx.recv().map_err(|_| WaitError::Closed)?);
                    true
                } else if let Ok(entry) = self.rx.recv_timeout(Duration::from_millis(25)) {
                    self.entries.push(entry);
                    true
                } else {
                    false
                };
                if try_more {
                    while let Ok(entry) = self.rx.try_recv() {
                        self.entries.push(entry);
                    }
                }

                self.p_buf.clear();
                for &(pid, _) in &self.entries {
                    self.p_buf
                        .push_str(itoa::Buffer::new().format(pid.0.get() as u32));
                }

                // TODO: use https://man.freebsd.org/cgi/man.cgi?query=kvm_getprocs instead of spawning
                // a process every time
                let mut child = std::process::Command::new("ps")
                    .args(["-p", &self.p_buf])
                    .stdout(Stdio::piped())
                    .spawn()
                    .context("Failed to spawn ps")?;

                self.stdout_buf.clear();
                child
                    .stdout
                    .take()
                    .context("Missing child stdout")?
                    .read_to_end(&mut self.stdout_buf)
                    .context("Failed to read ps output")?;

                child.wait().context("Failed to wait for ps")?;

                self.seen_buf.truncate(self.entries.len());
                self.seen_buf.fill(false);
                self.seen_buf.extend(std::iter::repeat_n(
                    false,
                    self.entries.len() - self.seen_buf.len(),
                ));
                for line in self.stdout_buf.split(|b| *b == b'\n').skip(1) {
                    if line.is_empty() {
                        continue;
                    }
                    let line = line.trim_ascii_start();
                    let Some((pid, _)) = line.split_once(|b| *b == b' ') else {
                        bad_output_dump(log, &self.stdout_buf);
                        return Err(anyhow!("Bad output from ps").into());
                    };
                    let pid = u32::from_ascii(pid)
                        .context("Bad output from ps")
                        .inspect_err(|_| {
                            bad_output_dump(log, &self.stdout_buf);
                        })?;
                    let Some(i) = self
                        .entries
                        .iter()
                        .position(|(other_pid, _)| other_pid.0.get() as u32 == pid)
                    else {
                        return Err(anyhow!("Bad output from ps: unknown pid {}", pid).into());
                    };
                    self.seen_buf[i] = true;
                }
                if let Some(i) = self.seen_buf.iter().position(|b| !*b) {
                    // there was a pid missing from the ps output, meaning the process is dead. return it.
                    let (_, data) = self.entries.swap_remove(i);
                    return Ok(data);
                }
            }
        }
    }

    fn bad_output_dump(log: &slog::Logger, stdout: &[u8]) {
        static DUMP: Once = Once::new();
        DUMP.call_once(|| match std::str::from_utf8(stdout) {
            Ok(s) => error!(log, "Bad output from ps: {:?}", s),
            Err(e) => error!(log, "Bad output from ps: {}", e),
        });
    }
}

#[cfg(target_os = "linux")]
mod sys {
    use std::marker::PhantomData;

    use crate::Pid;

    use super::{SubmitError, WaitError};

    pub fn wait_group<T>() -> (Submitter<T>, Waiter<T>) {
        (
            Submitter {
                _marker: PhantomData,
            },
            Waiter {
                _marker: PhantomData,
            },
        )
    }

    #[derive(Clone)]
    pub struct Submitter<T> {
        _marker: PhantomData<T>,
    }

    pub struct Waiter<T> {
        _marker: PhantomData<T>,
    }

    impl<T> Submitter<T> {
        pub fn submit(&self, pid: Pid, data: T) -> Result<(), SubmitError> {
            todo!()
        }
    }

    impl<T> Waiter<T> {
        pub fn wait_for_any(&mut self, log: &slog::Logger) -> Result<T, WaitError> {
            todo!()
        }
    }
}
