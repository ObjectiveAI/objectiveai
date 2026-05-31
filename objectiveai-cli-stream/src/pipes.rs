//! Per-agent named-pipe notify bridge.
//!
//! When the WS stream surfaces a new agent completion `response_id`,
//! we bind a socket for it at `${config_base_dir}/pipes/<agent_id>/socket`.
//! External processes that want to push a notification at that agent
//! connect to the socket and write NDJSON lines, one [`RichContent`]
//! per line. The reader task wraps each into an
//! [`AgentCompletionNotifyParams`] (with `response_id` set to the
//! pipe's agent id) and dispatches through the shared [`Notifier`].
//!
//! ## Path semantics
//!
//! Same layout on every platform: a filesystem path under
//! `${config_base_dir}/pipes/`. Each agent gets a folder at
//! `<agent_id>/` (slashes in the agent id become real subdirectories)
//! containing both the socket (fixed name `socket`) and a sibling
//! SQLite `db.sqlite` written by the log writer. On POSIX the socket
//! is a Unix domain socket; on Windows it's `AF_UNIX` (supported
//! since Windows 10 1803), which is also addressed by a filesystem
//! path, so we use `interprocess`'s `GenericFilePath` / `tokio`
//! generic socket code uniformly on both platforms — no cfg-gating.
//! Parent directories are auto-created. Stale socket files left
//! behind by a previous abnormal exit are unlinked on bind.
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
use objectiveai_sdk::agent::completions::message::{PipeAck, RichContent};
use objectiveai_sdk::agent::completions::request::AgentCompletionNotifyParams;
use objectiveai_sdk::cli::output::{Error, Handle, Level, Output};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
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
    let fs_path = pipes_root.join(agent_id).join("socket");
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
    ///
    /// `notif_tx` is a side-channel sender into the cli-stream's
    /// writer task. Every line the reader successfully parses as
    /// `RichContent` is fanned out to the API server via `notifier`
    /// (existing) AND pushed onto `notif_tx` so the writer task can
    /// log it under `agents/completions/request/notifications/...` and queue
    /// the matching DB row.
    pub async fn ensure_pipe(
        &self,
        agent_id: &str,
        response_id: &str,
        pipes_root: &Path,
        notifier: Notifier,
        notif_tx: tokio::sync::mpsc::UnboundedSender<(String, String, RichContent)>,
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
        let task_response_id = response_id.to_string();
        let task_notifier = notifier;
        let task_notif_tx = notif_tx;
        let task_handle = handle.clone();
        tokio::spawn(async move {
            run_listener(
                listener,
                task_agent_id,
                task_response_id,
                task_notifier,
                task_notif_tx,
                task_handle,
                cancel_rx,
            )
            .await;
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
    response_id: String,
    notifier: Notifier,
    notif_tx: tokio::sync::mpsc::UnboundedSender<(String, String, RichContent)>,
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
                        let notif_tx = notif_tx.clone();
                        let agent_id = agent_id.clone();
                        let response_id = response_id.clone();
                        let handle = handle.clone();
                        tokio::spawn(handle_connection(
                            conn, agent_id, response_id, notifier, notif_tx, handle,
                        ));
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
    response_id: String,
    notifier: Notifier,
    notif_tx: tokio::sync::mpsc::UnboundedSender<(String, String, RichContent)>,
    handle: Handle,
) {
    let (read_half, mut write_half) = conn.split();
    let reader = tokio::io::BufReader::new(read_half);
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
                let parse_msg = format!(
                    "pipe line for {agent_id:?} is not a valid RichContent JSON: {e}; line: {}",
                    truncate(trimmed, 200)
                );
                emit_error(&handle, parse_msg.clone()).await;
                // Tell the client too — broken-pipe on the ack write
                // (old client closed already) is swallowed silently.
                write_ack(&mut write_half, PipeAck::Error { message: parse_msg }).await;
                continue;
            }
        };
        // Side-channel into the cli-stream writer task so the
        // notification gets a log file + queued DB row. Best-effort —
        // if the writer task is gone, drop silently. `response_id` is
        // the target agent-completion's id, threaded down from the
        // pipe binding — same value the wire request body carries.
        let _ = notif_tx.send((agent_id.clone(), response_id.clone(), content.clone()));
        let params = AgentCompletionNotifyParams {
            response_id: response_id.clone(),
            content,
        };
        let ack = match notifier.notify(params).await {
            Ok(()) => PipeAck::Ok,
            Err(e) => {
                let msg = format!("notify dispatch for {agent_id:?}: {e}");
                emit_error(&handle, msg.clone()).await;
                PipeAck::Error { message: msg }
            }
        };
        write_ack(&mut write_half, ack).await;
    }
}

/// Serialize `ack` as one NDJSON line and write it back to the client.
/// Failures (typically a broken pipe — the client wrote a single line
/// and closed the half-duplex) are swallowed so the read loop can
/// continue for clients that send multiple lines per connection.
async fn write_ack<W>(writer: &mut W, ack: PipeAck)
where
    W: AsyncWriteExt + Unpin,
{
    let line = match serde_json::to_string(&ack) {
        Ok(s) => s,
        // `PipeAck` always serializes; the err arm is here for type
        // completeness only.
        Err(_) => return,
    };
    let _ = writer.write_all(line.as_bytes()).await;
    let _ = writer.write_all(b"\n").await;
    let _ = writer.flush().await;
}

async fn emit_error(handle: &Handle, message: String) {
    let out = Output::Error(Error {
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
