//! Per-agent named-pipe notify bridge.
//!
//! When the WS stream surfaces a new agent completion `response_id`,
//! we bind a socket for it at `${config_base_dir}/pipes/<agent_id>`.
//! External processes that want to push a notification at that agent
//! connect to the socket and write NDJSON lines, one [`RichContent`]
//! per line. The reader task wraps each into an
//! [`AgentCompletionNotifyParams`] (with `response_id` set to the
//! pipe's agent id) and dispatches through the shared [`Notifier`].
//!
//! ## Path semantics
//!
//! Same layout on every platform: a filesystem path under
//! `${config_base_dir}/pipes/`. Slashes in the agent id become real
//! subdirectories; the final segment is the socket file name. On
//! POSIX this is a Unix domain socket; on Windows it's `AF_UNIX`
//! (supported since Windows 10 1803), which is also addressed by a
//! filesystem path, so we use `interprocess`'s `GenericFilePath` /
//! `tokio` generic socket code uniformly on both platforms — no
//! cfg-gating. Parent directories are auto-created. Stale socket
//! files left behind by a previous abnormal exit are unlinked on
//! bind.
//!
//! ## Lifecycle
//!
//! [`PipeRegistry`] holds one cancel oneshot per active agent id.
//! [`ensure_pipe`] is idempotent — calling it again for an already-
//! tracked id is a no-op. [`PipeRegistry::shutdown`] fires every
//! cancel sender; each reader task drops its listener (which unlinks
//! the filesystem entry) and returns.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use interprocess::local_socket::tokio::{Listener, prelude::*};
use interprocess::local_socket::{GenericFilePath, ListenerOptions, Name, ToFsName};
use objectiveai_sdk::Notifier;
use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::agent::completions::request::AgentCompletionNotifyParams;
use objectiveai_sdk::cli::output::{Error, Handle, Level, Output};
use tokio::io::AsyncBufReadExt;
use tokio::sync::oneshot;

/// Compute the pipe address for `agent_id` under `pipes_root`.
///
/// Same layout on every platform: a filesystem path. `pipes_root` is
/// `${config_base_dir}/pipes`; the returned [`PipeAddress`] carries
/// both the `interprocess` [`Name`] used to bind and the underlying
/// [`PathBuf`] so the caller can pre-create parent dirs and unlink
/// stale socket files.
pub struct PipeAddress {
    pub name: Name<'static>,
    pub fs_path: PathBuf,
}

pub fn pipe_address_for_agent(
    pipes_root: &Path,
    agent_id: &str,
) -> Result<PipeAddress, String> {
    let fs_path = pipes_root.join(agent_id);
    let name = fs_path
        .clone()
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| format!("invalid pipe path for agent {agent_id:?}: {e}"))?
        .into_owned();
    Ok(PipeAddress { name, fs_path })
}

/// Tracks active per-agent pipe listener tasks. Clone-cheap.
#[derive(Default, Clone)]
pub struct PipeRegistry {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    cancellers: DashMap<String, oneshot::Sender<()>>,
}

impl PipeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a pipe for `agent_id` and spawn its reader task. No-op
    /// if a pipe for this id is already tracked. Errors during bind
    /// surface via `handle` and prevent the id from being inserted,
    /// so a later `ensure_pipe` call will retry.
    pub async fn ensure_pipe(
        &self,
        agent_id: &str,
        pipes_root: &Path,
        notifier: Notifier,
        handle: &Handle,
    ) {
        if self.inner.cancellers.contains_key(agent_id) {
            return;
        }

        let address = match pipe_address_for_agent(pipes_root, agent_id) {
            Ok(a) => a,
            Err(e) => {
                emit_error(handle, format!("pipe addr for {agent_id:?}: {e}")).await;
                return;
            }
        };

        if let Some(parent) = address.fs_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                emit_error(
                    handle,
                    format!("mkdir parent for {}: {e}", address.fs_path.display()),
                )
                .await;
                return;
            }
        }
        // Best-effort unlink — recover from a stale socket left
        // behind by a previous `kill -9`. Real failures surface from
        // the bind below.
        let _ = tokio::fs::remove_file(&address.fs_path).await;

        let listener = match ListenerOptions::new().name(address.name).create_tokio() {
            Ok(l) => l,
            Err(e) => {
                emit_error(
                    handle,
                    format!(
                        "bind pipe for {agent_id:?} at {}: {e}",
                        address.fs_path.display()
                    ),
                )
                .await;
                return;
            }
        };

        let (cancel_tx, cancel_rx) = oneshot::channel();
        let inserted = self
            .inner
            .cancellers
            .insert(agent_id.to_string(), cancel_tx);
        debug_assert!(inserted.is_none(), "ensure_pipe race: id already present");

        let task_agent_id = agent_id.to_string();
        let task_notifier = notifier;
        let task_handle = handle.clone();
        tokio::spawn(async move {
            run_listener(listener, task_agent_id, task_notifier, task_handle, cancel_rx).await;
        });
    }

    /// Fire every cancel and drop the registry. Reader tasks wake
    /// from their `tokio::select!`, drop their listeners (which
    /// unlinks the filesystem entry), and return.
    pub fn shutdown(&self) {
        // Drain into a Vec so we don't hold dashmap shard locks while
        // sending on the oneshots.
        let mut all: Vec<(String, oneshot::Sender<()>)> = Vec::new();
        let keys: Vec<String> = self
            .inner
            .cancellers
            .iter()
            .map(|kv| kv.key().clone())
            .collect();
        for k in keys {
            if let Some((id, tx)) = self.inner.cancellers.remove(&k) {
                all.push((id, tx));
            }
        }
        for (_id, tx) in all {
            let _ = tx.send(());
        }
    }
}

async fn run_listener(
    listener: Listener,
    agent_id: String,
    notifier: Notifier,
    handle: Handle,
    mut cancel: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut cancel => break,
            accept = listener.accept() => {
                match accept {
                    Ok(conn) => {
                        let notifier = notifier.clone();
                        let agent_id = agent_id.clone();
                        let handle = handle.clone();
                        tokio::spawn(handle_connection(conn, agent_id, notifier, handle));
                    }
                    Err(e) => {
                        emit_error(
                            &handle,
                            format!("pipe accept for {agent_id:?}: {e}"),
                        )
                        .await;
                        // Brief backoff so a hard-broken listener
                        // doesn't spin.
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        }
    }
}

async fn handle_connection(
    conn: interprocess::local_socket::tokio::Stream,
    agent_id: String,
    notifier: Notifier,
    handle: Handle,
) {
    let reader = tokio::io::BufReader::new(conn);
    let mut lines = reader.lines();
    loop {
        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => return,
            Err(e) => {
                emit_error(
                    &handle,
                    format!("pipe read for {agent_id:?}: {e}"),
                )
                .await;
                return;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let content: RichContent = match serde_json::from_str(trimmed) {
            Ok(c) => c,
            Err(e) => {
                emit_error(
                    &handle,
                    format!(
                        "pipe line for {agent_id:?} is not a valid RichContent JSON: {e}; line: {}",
                        truncate(trimmed, 200)
                    ),
                )
                .await;
                continue;
            }
        };
        let params = AgentCompletionNotifyParams {
            response_id: agent_id.clone(),
            content,
        };
        if let Err(e) = notifier.notify(params).await {
            emit_error(
                &handle,
                format!("notify dispatch for {agent_id:?}: {e}"),
            )
            .await;
        }
    }
}

async fn emit_error(handle: &Handle, message: String) {
    let out = Output::<serde_json::Value>::Error(Error {
        level: Level::Warn,
        fatal: false,
        message: serde_json::Value::String(message),
        agent_id: None,
    });
    out.emit(handle).await;
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
