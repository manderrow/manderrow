//! Ensures only one instance of the app runs at a time. Launching the app again brings the
//! existing instance to the front instead.

use std::io::{BufRead, BufReader, Write};

use anyhow::Context;
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{ListenerOptions, Name};

pub struct InstanceLock {
    listener: LocalSocketListener,
}

fn socket_name() -> std::io::Result<Name<'static>> {
    #[cfg(windows)]
    {
        use interprocess::local_socket::GenericNamespaced;
        // Named pipes are machine-wide, so scope the name to the user.
        let user = std::env::var("USERNAME").unwrap_or_default();
        format!("manderrow-instance-{user}")
            .to_ns_name::<GenericNamespaced>()
            .map(Name::into_owned)
    }
    #[cfg(not(windows))]
    {
        use interprocess::local_socket::GenericFilePath;
        // The runtime directory is private to the user.
        manderrow_paths::runtime_dir()
            .join("instance.sock")
            .to_fs_name::<GenericFilePath>()
            .map(Name::into_owned)
    }
}

/// Returns `None` if another instance is already running, after asking it to come to the front.
pub fn acquire() -> anyhow::Result<Option<InstanceLock>> {
    let name = socket_name().context("Failed to create the instance socket name")?;

    let activate = |name: Name<'_>| -> anyhow::Result<bool> {
        match LocalSocketStream::connect(name) {
            Ok(mut stream) => {
                stream
                    .write_all(b"activate\n")
                    .context("Failed to signal the running instance")?;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    };

    if activate(name.borrow())? {
        return Ok(None);
    }

    // Nothing answered, so any existing socket was left behind by an instance that crashed.
    match ListenerOptions::new()
        .name(name.borrow())
        .try_overwrite(true)
        .create_sync()
    {
        Ok(listener) => Ok(Some(InstanceLock { listener })),
        // another instance started listening in the meantime
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse && activate(name.borrow())? => Ok(None),
        Err(e) => Err(anyhow::Error::from(e).context("Failed to create the instance socket")),
    }
}

impl InstanceLock {
    /// Calls `on_activate` on a background thread whenever another instance is launched.
    pub fn listen(self, on_activate: impl Fn() + Send + 'static) {
        std::thread::Builder::new()
            .name("single-instance".into())
            .spawn(move || {
                for stream in self.listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let mut line = String::new();
                    if BufReader::new(stream).read_line(&mut line).is_ok()
                        && line.trim() == "activate"
                    {
                        on_activate();
                    }
                }
            })
            .expect("failed to spawn single-instance thread");
    }
}
