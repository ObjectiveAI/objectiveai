//! Per-agent outbound event broadcast bridge.
//!
//! For every `response_id` the chunk stream surfaces we bind one
//! Unix-domain socket at
//! `${config_base_dir}/pipes/<agent_instance_hierarchy>/events.sock`.
//! `agents/instances/read/subscribe` clients connect to that socket
//! and read NDJSON [`SubscribeEvent`] frames as they fan out from
//! the chunk-loop (log row written / stream end).
//!
//! ## Warnings
//!
//! Non-fatal bind / accept / serialize errors are surfaced as
//! `InstanceEmission::Warning { message }` items sent on the shared
//! emissions channel — never printed to stderr. The registry holds
//! a sender clone at construction time so detached listener tasks
//! can emit through the same stream.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use interprocess::local_socket::tokio::{Listener, prelude::*};
use interprocess::local_socket::{GenericFilePath, ListenerOptions, Name, ToFsName};
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::error::Error;
use crate::filesystem::logs::SubscribeEvent;
use crate::instance::InstanceEmission;

/// Channel type used by every emit site in this module.
pub type EmissionsTx = mpsc::Sender<Result<InstanceEmission, Error>>;

const OUTBOUND_BROADCAST_CAPACITY: usize = 1024;
const PROBE_TIMEOUT: Duration = Duration::from_millis(250);

enum BindOutcome {
    Bound(Listener),
    SlotTaken,
    Io(std::io::Error),
}

fn is_addr_in_use(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    if matches!(e.kind(), ErrorKind::AddrInUse | ErrorKind::AlreadyExists) {
        return true;
    }
    if let Some(code) = e.raw_os_error() {
        if cfg!(windows) && (code == 231 || code == 5) {
            return true;
        }
    }
    false
}

async fn bind_or_busy(address: &PipeAddress) -> BindOutcome {
    const MAX_ATTEMPTS: usize = 4;
    for _ in 0..MAX_ATTEMPTS {
        match ListenerOptions::new()
            .name(address.name.clone())
            .create_tokio()
        {
            Ok(l) => return BindOutcome::Bound(l),
            Err(e) if is_addr_in_use(&e) => {
                let probe_name = address.fs_path.clone().to_fs_name::<GenericFilePath>();
                let live = match probe_name {
                    Ok(n) => tokio::time::timeout(
                        PROBE_TIMEOUT,
                        interprocess::local_socket::tokio::Stream::connect(n),
                    )
                    .await
                    .is_ok_and(|r| r.is_ok()),
                    Err(_) => false,
                };
                if live {
                    return BindOutcome::SlotTaken;
                }
                let _ = tokio::fs::remove_file(&address.fs_path).await;
            }
            Err(e) => return BindOutcome::Io(e),
        }
    }
    BindOutcome::SlotTaken
}

pub struct PipeAddress {
    pub name: Name<'static>,
    pub fs_path: PathBuf,
}

pub fn events_address_for_agent(
    pipes_root: &Path,
    agent_instance_hierarchy: &str,
) -> Result<PipeAddress, String> {
    let fs_path = pipes_root.join(agent_instance_hierarchy).join("events.sock");
    let name = fs_path
        .clone()
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| {
            format!("invalid events pipe path for agent {agent_instance_hierarchy:?}: {e}")
        })?
        .into_owned();
    Ok(PipeAddress { name, fs_path })
}

/// Tracks active per-agent outbound listener tasks. Clone-cheap. The
/// constructor takes the emissions sender so every detached listener
/// task can surface warnings through the same stream.
#[derive(Clone)]
pub struct PipeRegistry {
    inner: Arc<Inner>,
}

struct Inner {
    outbound_cancellers: DashMap<String, oneshot::Sender<()>>,
    outbound_senders: DashMap<String, broadcast::Sender<SubscribeEvent>>,
    emissions_tx: EmissionsTx,
}

impl PipeRegistry {
    pub fn new(emissions_tx: EmissionsTx) -> Self {
        Self {
            inner: Arc::new(Inner {
                outbound_cancellers: DashMap::new(),
                outbound_senders: DashMap::new(),
                emissions_tx,
            }),
        }
    }

    pub fn emissions_tx(&self) -> &EmissionsTx {
        &self.inner.emissions_tx
    }

    pub async fn ensure_outbound_pipe(
        &self,
        agent_instance_hierarchy: &str,
        pipes_root: &Path,
    ) -> broadcast::Sender<SubscribeEvent> {
        if let Some(existing) = self.inner.outbound_senders.get(agent_instance_hierarchy) {
            return existing.clone();
        }

        let address = match events_address_for_agent(pipes_root, agent_instance_hierarchy) {
            Ok(a) => a,
            Err(e) => {
                emit_warning(
                    &self.inner.emissions_tx,
                    format!("outbound pipe addr for {agent_instance_hierarchy:?}: {e}"),
                )
                .await;
                let (tx, _) = broadcast::channel(OUTBOUND_BROADCAST_CAPACITY);
                return self.install_outbound_sender(agent_instance_hierarchy, tx);
            }
        };

        if let Some(parent) = address.fs_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                emit_warning(
                    &self.inner.emissions_tx,
                    format!(
                        "mkdir parent for outbound {}: {e}",
                        address.fs_path.display()
                    ),
                )
                .await;
                let (tx, _) = broadcast::channel(OUTBOUND_BROADCAST_CAPACITY);
                return self.install_outbound_sender(agent_instance_hierarchy, tx);
            }
        }
        let listener = match bind_or_busy(&address).await {
            BindOutcome::Bound(l) => l,
            BindOutcome::SlotTaken => {
                emit_warning(
                    &self.inner.emissions_tx,
                    format!(
                        "outbound pipe slot already taken for {agent_instance_hierarchy:?} at {}",
                        address.fs_path.display()
                    ),
                )
                .await;
                let (tx, _) = broadcast::channel(OUTBOUND_BROADCAST_CAPACITY);
                return self.install_outbound_sender(agent_instance_hierarchy, tx);
            }
            BindOutcome::Io(e) => {
                emit_warning(
                    &self.inner.emissions_tx,
                    format!(
                        "bind outbound pipe for {agent_instance_hierarchy:?} at {}: {e}",
                        address.fs_path.display()
                    ),
                )
                .await;
                let (tx, _) = broadcast::channel(OUTBOUND_BROADCAST_CAPACITY);
                return self.install_outbound_sender(agent_instance_hierarchy, tx);
            }
        };

        let (tx, _) = broadcast::channel::<SubscribeEvent>(OUTBOUND_BROADCAST_CAPACITY);
        let installed = self.install_outbound_sender(agent_instance_hierarchy, tx.clone());

        let (cancel_tx, cancel_rx) = oneshot::channel();
        let prev = self
            .inner
            .outbound_cancellers
            .insert(agent_instance_hierarchy.to_string(), cancel_tx);
        debug_assert!(
            prev.is_none(),
            "ensure_outbound_pipe race: id already present"
        );

        let task_agent_instance_hierarchy = agent_instance_hierarchy.to_string();
        let task_tx = tx;
        let task_emissions_tx = self.inner.emissions_tx.clone();
        tokio::spawn(async move {
            run_outbound_listener(
                listener,
                task_agent_instance_hierarchy,
                task_tx,
                task_emissions_tx,
                cancel_rx,
            )
            .await;
        });

        installed
    }

    fn install_outbound_sender(
        &self,
        agent_instance_hierarchy: &str,
        tx: broadcast::Sender<SubscribeEvent>,
    ) -> broadcast::Sender<SubscribeEvent> {
        let entry = self
            .inner
            .outbound_senders
            .entry(agent_instance_hierarchy.to_string())
            .or_insert(tx);
        entry.clone()
    }

    pub fn outbound_sender(
        &self,
        agent_instance_hierarchy: &str,
    ) -> Option<broadcast::Sender<SubscribeEvent>> {
        self.inner
            .outbound_senders
            .get(agent_instance_hierarchy)
            .map(|entry| entry.clone())
    }

    pub fn broadcast_stream_end(&self) {
        let senders: Vec<broadcast::Sender<SubscribeEvent>> = self
            .inner
            .outbound_senders
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        for tx in senders {
            let _ = tx.send(SubscribeEvent::StreamEnd);
        }
    }

    pub fn shutdown_outbound(&self) {
        let mut outbound_cancels: Vec<(String, oneshot::Sender<()>)> = Vec::new();
        let keys: Vec<String> = self
            .inner
            .outbound_cancellers
            .iter()
            .map(|kv| kv.key().clone())
            .collect();
        for k in keys {
            if let Some((id, tx)) = self.inner.outbound_cancellers.remove(&k) {
                outbound_cancels.push((id, tx));
            }
        }
        for (_id, tx) in outbound_cancels {
            let _ = tx.send(());
        }
        self.inner.outbound_senders.clear();
    }
}

async fn run_outbound_listener(
    listener: Listener,
    agent_instance_hierarchy: String,
    sender: broadcast::Sender<SubscribeEvent>,
    emissions_tx: EmissionsTx,
    mut cancel: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut cancel => break,
            accept = listener.accept() => {
                match accept {
                    Ok(conn) => {
                        let rx = sender.subscribe();
                        let agent_instance_hierarchy = agent_instance_hierarchy.clone();
                        let emissions_tx = emissions_tx.clone();
                        tokio::spawn(handle_outbound_connection(conn, agent_instance_hierarchy, rx, emissions_tx));
                    }
                    Err(e) => {
                        emit_warning(
                            &emissions_tx,
                            format!("outbound pipe accept for {agent_instance_hierarchy:?}: {e}"),
                        )
                        .await;
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        }
    }
}

async fn handle_outbound_connection(
    conn: interprocess::local_socket::tokio::Stream,
    agent_instance_hierarchy: String,
    mut rx: broadcast::Receiver<SubscribeEvent>,
    emissions_tx: EmissionsTx,
) {
    let (_read_half, mut write_half) = conn.split();
    loop {
        let event = match rx.recv().await {
            Ok(ev) => ev,
            Err(broadcast::error::RecvError::Closed) => return,
            Err(broadcast::error::RecvError::Lagged(_)) => return,
        };
        let is_end = matches!(event, SubscribeEvent::StreamEnd);
        let line = match serde_json::to_string(&event) {
            Ok(s) => s,
            Err(e) => {
                emit_warning(
                    &emissions_tx,
                    format!("serialize outbound event for {agent_instance_hierarchy:?}: {e}"),
                )
                .await;
                continue;
            }
        };
        if write_half.write_all(line.as_bytes()).await.is_err() {
            return;
        }
        if write_half.write_all(b"\n").await.is_err() {
            return;
        }
        if write_half.flush().await.is_err() {
            return;
        }
        if is_end {
            return;
        }
    }
}

async fn emit_warning(tx: &EmissionsTx, message: String) {
    let _ = tx.send(Ok(InstanceEmission::Warning { message })).await;
}
