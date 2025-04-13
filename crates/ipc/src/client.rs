use std::sync::Mutex;

use ipc_channel::ipc::{IpcError, IpcReceiver, IpcSender, OpaqueIpcSender};

use crate::{C2SMessage, S2CMessage};

/// Inter-process communication.
pub struct Ipc {
    c2s_tx: Mutex<Option<OpaqueIpcSender>>,
    s2c_rx: Mutex<IpcReceiver<S2CMessage>>,
}

impl Ipc {
    pub fn new(c2s_tx: IpcSender<C2SMessage>, s2c_rx: IpcReceiver<S2CMessage>) -> Self {
        Self {
            c2s_tx: Mutex::new(Some(c2s_tx.to_opaque())),
            s2c_rx: s2c_rx.into(),
        }
    }

    pub fn send(&self, message: &C2SMessage) -> Result<(), SendError> {
        // god awful API provided by ipc-channel
        let mut lock = self.c2s_tx.lock().map_err(|_| SendError::Poisoned)?;
        if let Some(c2s_tx) = lock.take() {
            let c2s_tx = c2s_tx.to();
            let r = c2s_tx.send(message);
            *lock = Some(c2s_tx.to_opaque());
            r.map_err(Into::into)
        } else {
            // this is unreachable, but I don't want to panic
            Ok(())
        }
    }

    pub fn recv(&self) -> Result<S2CMessage, RecvError> {
        self.s2c_rx
            .lock()
            .map_err(|_| RecvError::Poisoned)?
            .recv()
            .map_err(|e| match e {
                IpcError::Bincode(e) => RecvError::Decode(e),
                IpcError::Io(e) => RecvError::Io(e),
                IpcError::Disconnected => RecvError::Disconnected,
            })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("failed to encode message: {0}")]
    Encode(#[from] bincode::Error),
    #[error("lock is poisoned")]
    Poisoned,
}

#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    #[error("failed to decode message: {0}")]
    Decode(#[from] bincode::Error),
    #[error("encountered I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lock is poisoned")]
    Poisoned,
    #[error("the channel is disconnected")]
    Disconnected,
}
