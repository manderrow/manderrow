pub mod commands;

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::atomic::AtomicU64;

use anyhow::{Context, Result};
use manderrow_ipc::ipc_channel::ipc::{IpcReceiver, IpcSender};
use manderrow_process_util::Pid;
use parking_lot::{Mutex, RwLock};
use slog::{debug, error, warn};
use tauri::{AppHandle, Emitter};

pub use manderrow_ipc::*;
use triomphe::Arc;

pub const EVENT_TARGET: &str = "main";
pub const EVENT_NAME: &str = "ipc_message";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ConnectionId(u64);

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl slog::Value for ConnectionId {
    fn serialize(
        &self,
        record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        self.0.serialize(record, key, serializer)
    }
}

#[derive(Clone)]
pub struct IpcConnection(Arc<Mutex<IpcConnectionState>>);

impl IpcConnection {
    pub async fn send_async(&self, msg: S2CMessage) -> Result<(), SendError> {
        let state = self.0.lock();
        match &*state {
            IpcConnectionState::InternalConnecting => Err(SendError::IncompleteConnection),
            IpcConnectionState::Internal(_) => {
                // by mapping the guard, we take ExternalIpcConnection out of
                // the equation and the future implements Send
                parking_lot::MutexGuard::map(state, |state| match state {
                    IpcConnectionState::Internal(conn) => conn,
                    _ => unreachable!(),
                })
                .s2c_tx
                .send(msg)
                .await
                .map_err(|_| SendError::ConnectionClosed)
            }
            IpcConnectionState::ExternalConnecting => Err(SendError::IncompleteConnection),
            IpcConnectionState::External(conn) => {
                let s2c_tx = conn.s2c_tx.clone();
                drop(state);
                tokio::task::spawn_blocking(move || {
                    s2c_tx.send(msg).map_err(SendError::ExternalSendError)
                })
                .await
                .expect("task panicked")
            }
        }
    }

    pub fn kill_process(&self, log: &slog::Logger) -> Result<(), KillError> {
        let state = self.0.lock();
        match &*state {
            IpcConnectionState::InternalConnecting
            | IpcConnectionState::Internal(_)
            | IpcConnectionState::ExternalConnecting => Err(KillError::IncompleteConnection),
            IpcConnectionState::External(conn) => {
                // TODO: kill button tries soft first, then second click tries hard
                conn.pid.kill(log, true)?;
                Ok(())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Connection is incomplete")]
    IncompleteConnection,
    #[error("External connection send failed: {0}")]
    ExternalSendError(bincode::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum KillError {
    #[error("Connection is incomplete")]
    IncompleteConnection,
    #[error("Failed to kill the process: {0}")]
    Other(#[from] anyhow::Error),
}

struct InternalIpcConnection {
    s2c_tx: tokio::sync::mpsc::Sender<S2CMessage>,
}

struct ExternalIpcConnection {
    s2c_tx: IpcSender<S2CMessage>,
    /// The id of the receiver in the set.
    c2s_rx: u64,
    pid: Pid,
}

enum IpcConnectionState {
    InternalConnecting,
    Internal(InternalIpcConnection),
    ExternalConnecting,
    External(ExternalIpcConnection),
}

#[derive(serde::Deserialize, serde::Serialize)]
enum ManagementEvent {
    ExternalRegistration {
        id: ConnectionId,
        c2s_rx: IpcReceiver<C2SMessage>,
        s2c_tx: String,
        pid: Pid,
    },
    Death {
        id: ConnectionId,
    },
}

#[derive(Clone, serde::Serialize)]
pub struct IdentifiedC2SMessage<'a> {
    #[serde(rename = "connId")]
    pub conn_id: ConnectionId,
    #[serde(flatten)]
    pub msg: &'a C2SMessage,
}

pub struct IpcState {
    next_connection_id: AtomicU64,
    connections: Arc<RwLock<HashMap<ConnectionId, IpcConnection>>>,
    receiver_handle: std::thread::JoinHandle<()>,
    mgmt_tx: Arc<Mutex<IpcSender<ManagementEvent>>>,
    death_wait_submitter: manderrow_process_util::wait_group::Submitter<ConnectionId>,
}

impl IpcState {
    pub fn new(app: AppHandle, log: slog::Logger) -> Self {
        let connections: Arc<RwLock<HashMap<ConnectionId, IpcConnection>>> = Default::default();
        // we use an IPC channel here to enable the receiver thread to efficiently
        // receive messages from this and the external channels at the same time
        let (mgmt_tx, mgmt_rx) =
            ipc_channel::ipc::channel::<ManagementEvent>().expect("failed to create ipc");
        let (death_wait_submitter, mut death_waiter) =
            manderrow_process_util::wait_group::wait_group();
        {
            let log = log.clone();
            let mgmt_tx = mgmt_tx.clone();
            std::thread::Builder::new()
                .name("ipc-reaper".into())
                .spawn(move || loop {
                    match death_waiter.wait_for_any(&log) {
                        Ok(id) => {
                            if let Err(e) = mgmt_tx.send(ManagementEvent::Death { id }) {
                                error!(
                                    log,
                                    "Failed to send death event to ipc-receiver thread: {}", e
                                );
                            }
                        }
                        Err(manderrow_process_util::wait_group::WaitError::Closed) => break,
                        Err(manderrow_process_util::wait_group::WaitError::Other(e)) => {
                            error!(log, "{}", e);
                        }
                    }
                })
                .expect("failed to spawn ipc-reaper thread");
        }
        Self {
            next_connection_id: AtomicU64::new(0),
            connections: connections.clone(),
            receiver_handle: std::thread::Builder::new()
                .name("ipc-receiver".into())
                .spawn(move || {
                    let mut rx_to_id = HashMap::<u64, ConnectionId>::new();
                    let mut set = ipc_channel::ipc::IpcReceiverSet::new().expect("failed to create IpcReceiverSet");
                    let mgmt_rx = set.add(mgmt_rx).expect("Failed to add management receiver to the set");
                    while let Ok(messages) = set.select() {
                        for msg in messages {
                            use ipc_channel::ipc::IpcSelectionResult::*;
                            match msg {
                                MessageReceived(id, msg) if id == mgmt_rx => {
                                    let event = match msg
                                        .to::<ManagementEvent>(
                                    ) {
                                        Ok(t) => t,
                                        Err(e) => {
                                            error!(log, "Bad data sent across in-process IPC! {e}");
                                            continue;
                                        }
                                    };
                                    match event {
                                        ManagementEvent::ExternalRegistration { id, c2s_rx, s2c_tx, pid } => {
                                            let mut connections = connections
                                                .upgradable_read();
                                            let Some(conn) = connections
                                                .get(&id) else {
                                                    warn!(log, "Received registration request for unregistered connection"; "conn_id" => id);
                                                    continue;
                                                };
                                            let mut state = conn.0.lock();
                                            if !matches!(*state, IpcConnectionState::ExternalConnecting) {
                                                warn!(log, "Inconsistent internal state for connection {}", match *state {
                                                    IpcConnectionState::InternalConnecting => "InternalConnecting",
                                                    IpcConnectionState::Internal(_) => "Internal",
                                                    IpcConnectionState::ExternalConnecting => unreachable!(),
                                                    IpcConnectionState::External(_) => "External",
                                                }; "conn_id" => id);
                                                continue;
                                            }
                                            let s2c_tx = match IpcSender::connect(s2c_tx.clone()) {
                                                Ok(t) => t,
                                                Err(e) => {
                                                    drop(state);
                                                    warn!(log, "Failed to connect to s2c channel: {}", e; "conn_id" => id, "s2c_tx" => ?s2c_tx);
                                                    connections.with_upgraded(|connections| connections.remove(&id));
                                                    continue;
                                                }
                                            };
                                            if let Err(e) = s2c_tx.send(S2CMessage::Connect) {
                                                error!(log, "Failed to send s2c connect message: {}", e);
                                            }
                                            let c2s_rx = match set.add(c2s_rx) {
                                                Ok(t) => t,
                                                Err(e) => {
                                                    drop(state);
                                                    error!(log, "Failed to register in-process IPC! {e}"; "conn_id" => id);
                                                    connections.with_upgraded(|connections| connections.remove(&id));
                                                    continue;
                                                }
                                            };
                                            *state = IpcConnectionState::External(ExternalIpcConnection { s2c_tx, c2s_rx, pid });
                                            rx_to_id.insert(c2s_rx, id);
                                        }
                                        ManagementEvent::Death { id } => {
                                            let mut connections = connections.write();
                                            let Some(conn) = connections
                                                .remove(&id) else {
                                                    debug!(log, "Received death event for unregistered connection"; "conn_id" => id.0);
                                                    continue;
                                                };
                                            let state = conn.0.lock();
                                            match &*state {
                                                IpcConnectionState::External(ExternalIpcConnection { c2s_rx, .. }) => {
                                                    rx_to_id.remove(c2s_rx);
                                                }
                                                _ => {}
                                            }
                                            if let Err(e) = app.emit_to(EVENT_TARGET, "ipc_closed", id) {
                                                error!(log, "Failed to emit ipc_closed event to {}: {}", EVENT_TARGET, e; "conn_id" => id.0);
                                            }
                                        }
                                    }
                                }
                                MessageReceived(rx, msg) => {
                                    let Some(&id) = rx_to_id.get(&rx) else {
                                        warn!(log, "Received message for unknown receiver: {}", rx);
                                        continue;
                                    };
                                    let msg = match msg.to::<C2SMessage>() {
                                        Ok(t) => t,
                                        Err(e) => {
                                            warn!(log, "Bad data received from {}, receiver {}: {}", id, rx, e);
                                            connections.write().remove(&id);
                                            rx_to_id.remove(&rx);
                                            continue;
                                        }
                                    };
                                    let connections = connections.read();
                                    let Some(conn) = connections.get(&id) else {
                                        warn!(log, "Inconsistent internal state for connection (unknown)"; "conn_id" => id, "rx" => rx);
                                        continue;
                                    };
                                    let state = conn.0.lock();
                                    match &*state {
                                        IpcConnectionState::InternalConnecting | IpcConnectionState::Internal(_) | IpcConnectionState::ExternalConnecting => {
                                            warn!(log, "Inconsistent internal state for connection (not External)"; "conn_id" => id, "rx" => rx);
                                            continue;
                                        }
                                        IpcConnectionState::External(_) => {}
                                    }

                                    if let Err(e) = app.emit_to(EVENT_TARGET, EVENT_NAME, IdentifiedC2SMessage { conn_id: id, msg: &msg }) {
                                        error!(log, "Failed to emit ipc_message event to {}: {}", EVENT_TARGET, e; "conn_id" => id, "rx" => rx);
                                    }
                                }
                                ChannelClosed(rx) => {
                                    let Some(&id) = rx_to_id.get(&rx) else {
                                        warn!(log, "Received message for unknown receiver: {}", rx);
                                        continue;
                                    };
                                    connections.write().remove(&id);
                                    rx_to_id.remove(&rx);
                                    if let Err(e) = app.emit_to(EVENT_TARGET, "ipc_closed", id) {
                                        error!(log, "Failed to emit ipc_closed event to {}: {}", EVENT_TARGET, e; "conn_id" => id, "rx" => rx);
                                    }
                                }
                            }
                        }
                    }
                })
                .expect("failed to spawn ipc-receiver thread"),
            mgmt_tx: Arc::new(mgmt_tx.into()),
            death_wait_submitter,
        }
    }

    pub fn alloc(&self) -> ConnectionId {
        let id = ConnectionId(
            self.next_connection_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        self.connections.write().insert(
            id,
            IpcConnection(Arc::new(Mutex::new(IpcConnectionState::InternalConnecting))),
        );
        id
    }

    pub fn connect(
        &self,
        conn_id: ConnectionId,
        app: AppHandle,
    ) -> Result<InProcessIpc, ConnectError> {
        let (tx, rx) = tokio::sync::mpsc::channel::<S2CMessage>(1);
        *self
            .get_conn(conn_id)
            .ok_or(ConnectError::NoSuchConnection(conn_id))?
            .0
            .lock() = IpcConnectionState::Internal(InternalIpcConnection { s2c_tx: tx });
        Ok(InProcessIpc {
            conn_id,
            s2c_rx: rx,
            app,
        })
    }

    pub fn get_conn(&self, conn_id: ConnectionId) -> Option<IpcConnection> {
        self.connections.read().get(&conn_id).cloned()
    }

    pub fn get_conns(&self) -> Vec<ConnectionId> {
        self.connections.read().keys().copied().collect()
    }

    /// The returned string should be passed to [`IpcSender::<C2SMessage>::connect`].
    pub fn spawn_external(
        &self,
        log: slog::Logger,
        app: AppHandle,
        conn_id: ConnectionId,
    ) -> Result<String, SpawnError> {
        *self
            .get_conn(conn_id)
            .ok_or(SpawnError::NoSuchConnection(conn_id))?
            .0
            .lock() = IpcConnectionState::ExternalConnecting;

        let log = log.new(slog::o!("conn_id" => conn_id.0));
        let (server, name) = ipc_channel::ipc::IpcOneShotServer::<C2SMessage>::new()?;

        let connections = self.connections.clone();
        let mgmt_tx = self.mgmt_tx.clone();
        let death_wait_submitter = self.death_wait_submitter.clone();

        std::thread::Builder::new()
            .name(format!("ipc-receiver-server-{}", name))
            .spawn(move || {
                let (c2s_rx, msg) = match server.accept() {
                    Ok(t) => t,
                    Err(e) => {
                        error!(log, "Failed to accept IPC connection: {}", e);
                        return;
                    }
                };
                _ = app.emit_to(
                    EVENT_TARGET,
                    EVENT_NAME,
                    IdentifiedC2SMessage { conn_id, msg: &msg },
                );
                if let C2SMessage::Connect { s2c_tx, pid } = msg {
                    let pid = Pid::from_raw(pid);
                    if let Err(e) = mgmt_tx.lock().send(ManagementEvent::ExternalRegistration {
                        id: conn_id,
                        c2s_rx,
                        s2c_tx,
                        pid,
                    }) {
                        error!(
                            log,
                            "Failed to send registration request for connection: {}", e
                        );
                    }
                    if let Err(e) = death_wait_submitter.submit(pid, conn_id) {
                        error!(log, "Failed to send submit pid+id to reaper: {}", e);
                    }
                } else {
                    warn!(log, "Bad connect message: {:?}", msg);
                    connections.write().remove(&conn_id);
                }
            })?;
        Ok(name)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("No such connection {}", .0.0)]
    NoSuchConnection(ConnectionId),
    #[error("Wrong state of connection {}", .0.0)]
    WrongConnectionState(ConnectionId),
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("No such connection {}", .0.0)]
    NoSuchConnection(ConnectionId),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct InProcessIpc {
    conn_id: ConnectionId,
    s2c_rx: tokio::sync::mpsc::Receiver<S2CMessage>,
    app: AppHandle,
}

impl InProcessIpc {
    pub async fn send(&self, message: C2SMessage) -> Result<()> {
        let app = self.app.clone();
        let conn_id = self.conn_id;
        Ok(tokio::task::spawn_blocking(move || {
            app.emit_to(
                EVENT_TARGET,
                EVENT_NAME,
                IdentifiedC2SMessage {
                    conn_id,
                    msg: &message,
                },
            )
        })
        .await??)
    }

    pub async fn recv(&mut self) -> Result<S2CMessage> {
        Ok(self.s2c_rx.recv().await.context("Channel closed")?)
    }

    pub async fn prompt_patient<T: Send>(
        &mut self,
        translation_key: impl Into<String>,
        message: Option<String>,
        message_args: Option<HashMap<String, serde_json::Value>>,
        fixes: impl IntoIterator<Item = DoctorFix<T>>,
    ) -> Result<T>
    where
        T: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let (mut receiver, msg) =
            PatientChoiceReceiver::new(translation_key, message, message_args, fixes);
        self.send(msg).await?;
        loop {
            match receiver.process(self.recv().await?)? {
                ControlFlow::Break(choice) => return Ok(choice),
                ControlFlow::Continue(r) => receiver = r,
            }
        }
    }
}
