use std::sync::Mutex;

use ipc_channel::ipc::{IpcError, IpcReceiver, IpcSender};

use crate::{C2SMessage, S2CMessage};

/// Inter-process communication.
pub struct Ipc {
    pub c2s_tx: Mutex<IpcSender<C2SMessage>>,
    pub s2c_rx: Mutex<IpcReceiver<S2CMessage>>,
}

impl Ipc {
    pub fn send(&self, message: C2SMessage) -> Result<(), SendError> {
        Ok(self
            .c2s_tx
            .lock()
            .map_err(|_| SendError::Poisoned)?
            .send(message)?)
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
