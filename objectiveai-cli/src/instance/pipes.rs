//! Per-agent named-pipe notify bridge.
//!
//! When the WS stream surfaces a new agent completion `response_id`,
//! we bind a socket for it at `${config_base_dir}/pipes/<agent_instance_hierarchy>/socket`.
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
//! `<agent_instance_hierarchy>/` (slashes in the agent id become real subdirectories)
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
use std::time::Duration;

use dashmap::DashMap;
use interprocess::local_socket::tokio::{Listener, prelude::*};
use interprocess::local_socket::{GenericFilePath, ListenerOptions, Name, ToFsName};
use objectiveai_sdk::Notifier;
use objectiveai_sdk::agent::completions::message::{PipeAck, RichContent};
use objectiveai_sdk::agent::completions::request::AgentCompletionNotifyParams;
use objectiveai_sdk::cli::output::{Error, Handle, Level, Output};
use objectiveai_sdk::filesystem::logs::SubscribeEvent;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, oneshot};

/// Buffer depth for the per-agent outbound `events.sock` broadcast
/// channel. Generous so a slow subscriber doesn't lose events under
/// burst load; a subscriber that lags past this gets a `Lagged` from
/// `broadcast::Receiver::recv` and disconnects, mirroring the pipe-
/// disappearance case (subscribe retries from the top).
const OUTBOUND_BROADCAST_CAPACITY: usize = 1024;

/// Timeout for the connect-probe in [`bind_or_busy`]. Short — a
/// live listener accepts immediately; a stale file produces an
/// instant refused/notfound. Generous enough to cover loaded
/// systems but short enough that a real `kill -9` recovery doesn't
/// pay a perceptible wait.
const PROBE_TIMEOUT: Duration = Duration::from_millis(250);

/// Outcome of [`Self::ensure_pipe`] / [`Self::try_acquire_pipe`].
/// The `Live` variant is the admission-gate signal — a second caller
/// for the same agent identity must back out. `Io` is the existing
/// degraded-bind path: emit a warning, keep going (today's behaviour
/// preserved for non-conflict bind errors like permission denied).
#[derive(Debug)]
pub enum BindStatus {
    /// The socket path is currently owned by a live listener — the
    /// caller has lost the admission race. Surface as the
    /// `SLOT_TAKEN` exit code so the wrapper CLI can retry.
    Live,
    /// Path/permission/FS error during bind. Already logged via
    /// `handle.emit`; caller can treat as degraded same as today.
    Io,
}

/// Internal outcome of a single [`bind_or_busy`] attempt.
enum BindOutcome {
    Bound(Listener),
    SlotTaken,
    Io(std::io::Error),
}

/// True if `e` is an "address already in use" / "pipe busy" surface
/// from `ListenerOptions::create_tokio`. We accept the standard
/// portable kind plus Windows's pipe-busy raw code (231) and the
/// AF_UNIX-on-Windows access-denied surface (5), which is what
/// `interprocess` reports when the name collides with a live owner.
fn is_addr_in_use(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    if matches!(e.kind(), ErrorKind::AddrInUse | ErrorKind::AlreadyExists) {
        return true;
    }
    if let Some(code) = e.raw_os_error() {
        // Windows: ERROR_PIPE_BUSY = 231, ERROR_ACCESS_DENIED = 5.
        // Unix: EADDRINUSE is already covered by the kind check above.
        if cfg!(windows) && (code == 231 || code == 5) {
            return true;
        }
    }
    false
}

/// Atomically claim ownership of `address`, or report the slot as
/// `SlotTaken` if a live listener already holds it.
///
/// Protocol: try to bind. On `AddrInUse`, connect-probe — if the
/// probe succeeds, the path is owned by a live listener and we
/// return `SlotTaken`. If the probe fails (stale file from a prior
/// crash), unlink and loop. Concurrent stale-claimers all funnel
/// through the OS-level bind atomicity at the next iteration: at
/// most one of their `create_tokio()` calls wins, the others probe
/// the new live owner and bail correctly.
async fn bind_or_busy(address: &PipeAddress) -> BindOutcome {
    const MAX_ATTEMPTS: usize = 4;
    for _ in 0..MAX_ATTEMPTS {
        match ListenerOptions::new()
            .name(address.name.clone())
            .create_tokio()
        {
            Ok(l) => return BindOutcome::Bound(l),
            Err(e) if is_addr_in_use(&e) => {
                // Probe: does a live listener answer at this path?
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
                // Stale — best-effort unlink and retry. Two
                // concurrent stale-claimers can both reach this
                // branch; only one of their next `create_tokio()`
                // calls succeeds. The other loops back, probes the
                // new live owner, and returns SlotTaken.
                let _ = tokio::fs::remove_file(&address.fs_path).await;
            }
            Err(e) => return BindOutcome::Io(e),
        }
    }
    // Pathological thrash — treat as taken so the caller backs off.
    BindOutcome::SlotTaken
}

/// Compute the pipe address for `agent_instance_hierarchy` under `pipes_root`.
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

pub fn pipe_address_for_agent(pipes_root: &Path, agent_instance_hierarchy: &str) -> Result<PipeAddress, String> {
    let fs_path = pipes_root.join(agent_instance_hierarchy).join("socket");
    let name = fs_path
        .clone()
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| format!("invalid pipe path for agent {agent_instance_hierarchy:?}: {e}"))?
        .into_owned();
    Ok(PipeAddress { name, fs_path })
}

/// Sibling of [`pipe_address_for_agent`] for the outbound
/// `events.sock` endpoint cli-stream writes [`SubscribeEvent`]
/// NDJSON lines to. Same per-agent directory, different leaf name —
/// so the existing cleanup-on-Drop / shutdown story covers both
/// without extra plumbing.
pub fn events_address_for_agent(pipes_root: &Path, agent_instance_hierarchy: &str) -> Result<PipeAddress, String> {
    let fs_path = pipes_root.join(agent_instance_hierarchy).join("events.sock");
    let name = fs_path
        .clone()
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| format!("invalid events pipe path for agent {agent_instance_hierarchy:?}: {e}"))?
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
    /// Listeners pre-bound by [`PipeRegistry::try_acquire_pipe`] but
    /// not yet handed to a reader task (because `notifier` / `notif_tx`
    /// weren't ready yet). [`PipeRegistry::ensure_pipe`] consumes
    /// from here first, falling back to a fresh `bind_or_busy` if
    /// absent. This is the handoff between the eager admission probe
    /// (which fires at endpoint entry, before the API stream opens)
    /// and the per-chunk reader spawn (which fires inside
    /// `run_chunk_loop`).
    pending_listeners: DashMap<String, Listener>,
    /// Cancellers for the outbound `events.sock` listeners — one per
    /// agent. Tracked separately from the inbound cancellers so the
    /// two pipes have independent lifecycles (e.g. a malformed
    /// outbound bind doesn't deny the inbound side).
    outbound_cancellers: DashMap<String, oneshot::Sender<()>>,
    /// Per-agent broadcast sender. `writer_loop` looks these up to
    /// fan a `Row` / `StreamEnd` out to every subscriber currently
    /// connected to the corresponding `events.sock`. Subscribers
    /// connect to the AF_UNIX socket directly; the listener task
    /// `subscribe`s a fresh `broadcast::Receiver` per accepted
    /// connection, so a late-joining subscriber only sees events that
    /// landed AFTER its connect — the exact invariant the subscribe
    /// CLI relies on (open listener before first DB query).
    outbound_senders: DashMap<String, broadcast::Sender<SubscribeEvent>>,
}

impl PipeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Eager admission probe — atomically bind the inbound socket
    /// for `agent_instance_hierarchy` and stash the resulting [`Listener`] in
    /// `pending_listeners`. The matching [`Self::ensure_pipe`] call
    /// (which fires per-chunk from `run_chunk_loop` once `notifier` /
    /// `notif_tx` are ready) consumes the stashed listener and spawns
    /// the reader task on top.
    ///
    /// Returns `Err(BindStatus::Live)` when the slot is currently
    /// owned by another live listener — the wrapper CLI translates
    /// that to the `SLOT_TAKEN` exit code and recursively retries.
    /// Idempotent: re-calls for an already-pending or already-bound
    /// id are `Ok(())` no-ops.
    pub async fn try_acquire_pipe(
        &self,
        agent_instance_hierarchy: &str,
        pipes_root: &Path,
        handle: &Handle,
    ) -> Result<(), BindStatus> {
        if self.inner.cancellers.contains_key(agent_instance_hierarchy)
            || self.inner.pending_listeners.contains_key(agent_instance_hierarchy)
        {
            return Ok(());
        }

        let address = match pipe_address_for_agent(pipes_root, agent_instance_hierarchy) {
            Ok(a) => a,
            Err(e) => {
                emit_error(handle, format!("pipe addr for {agent_instance_hierarchy:?}: {e}")).await;
                return Err(BindStatus::Io);
            }
        };

        if let Some(parent) = address.fs_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                emit_error(
                    handle,
                    format!("mkdir parent for {}: {e}", address.fs_path.display()),
                )
                .await;
                return Err(BindStatus::Io);
            }
        }

        match bind_or_busy(&address).await {
            BindOutcome::Bound(l) => {
                self.inner
                    .pending_listeners
                    .insert(agent_instance_hierarchy.to_string(), l);
                Ok(())
            }
            BindOutcome::SlotTaken => Err(BindStatus::Live),
            BindOutcome::Io(e) => {
                emit_error(
                    handle,
                    format!(
                        "bind pipe for {agent_instance_hierarchy:?} at {}: {e}",
                        address.fs_path.display()
                    ),
                )
                .await;
                Err(BindStatus::Io)
            }
        }
    }

    /// Bind a pipe for `agent_instance_hierarchy` and spawn its reader task. No-op
    /// if a pipe for this id is already tracked. Returns
    /// `Err(BindStatus::Live)` when the slot is owned by another
    /// live listener — the run_chunk_loop caller propagates that to
    /// a `SLOT_TAKEN` process exit. `Err(BindStatus::Io)` is the
    /// existing degraded-bind path (path/permission errors): emit
    /// a warning, prevent insertion so a later call retries, and
    /// let the stream continue.
    ///
    /// `notif_tx` is a side-channel sender into the cli-stream's
    /// writer task. Every line the reader successfully parses as
    /// `RichContent` is fanned out to the API server via `notifier`
    /// (existing) AND pushed onto `notif_tx` so the writer task can
    /// log it under `agents/completions/request/notifications/...` and queue
    /// the matching DB row.
    pub async fn ensure_pipe(
        &self,
        agent_instance_hierarchy: &str,
        response_id: &str,
        pipes_root: &Path,
        notifier: Notifier,
        notif_tx: tokio::sync::mpsc::UnboundedSender<(String, String, RichContent)>,
        handle: &Handle,
    ) -> Result<(), BindStatus> {
        if self.inner.cancellers.contains_key(agent_instance_hierarchy) {
            return Ok(());
        }

        // Take the pre-bound listener if `try_acquire_pipe` already
        // claimed this slot. Otherwise bind fresh.
        let listener = if let Some((_, l)) = self.inner.pending_listeners.remove(agent_instance_hierarchy) {
            l
        } else {
            let address = match pipe_address_for_agent(pipes_root, agent_instance_hierarchy) {
                Ok(a) => a,
                Err(e) => {
                    emit_error(handle, format!("pipe addr for {agent_instance_hierarchy:?}: {e}")).await;
                    return Err(BindStatus::Io);
                }
            };

            if let Some(parent) = address.fs_path.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    emit_error(
                        handle,
                        format!("mkdir parent for {}: {e}", address.fs_path.display()),
                    )
                    .await;
                    return Err(BindStatus::Io);
                }
            }

            match bind_or_busy(&address).await {
                BindOutcome::Bound(l) => l,
                BindOutcome::SlotTaken => return Err(BindStatus::Live),
                BindOutcome::Io(e) => {
                    emit_error(
                        handle,
                        format!(
                            "bind pipe for {agent_instance_hierarchy:?} at {}: {e}",
                            address.fs_path.display()
                        ),
                    )
                    .await;
                    return Err(BindStatus::Io);
                }
            }
        };

        let (cancel_tx, cancel_rx) = oneshot::channel();
        let inserted = self
            .inner
            .cancellers
            .insert(agent_instance_hierarchy.to_string(), cancel_tx);
        debug_assert!(inserted.is_none(), "ensure_pipe race: id already present");

        let task_agent_instance_hierarchy = agent_instance_hierarchy.to_string();
        let task_response_id = response_id.to_string();
        let task_notifier = notifier;
        let task_notif_tx = notif_tx;
        let task_handle = handle.clone();
        tokio::spawn(async move {
            run_listener(
                listener,
                task_agent_instance_hierarchy,
                task_response_id,
                task_notifier,
                task_notif_tx,
                task_handle,
                cancel_rx,
            )
            .await;
        });
        Ok(())
    }

    /// Bind the outbound `events.sock` for `agent_instance_hierarchy` and spawn its
    /// fanout listener task. Idempotent: returns a clone of the
    /// existing `broadcast::Sender` if one is already tracked.
    ///
    /// On bind failure (rare — only path validity / FS errors), this
    /// surfaces a warning via `handle` and returns a degraded
    /// sender that's only used internally (no listener will ever
    /// consume from its receivers). Callers can still `send()` on it
    /// without panicking; subscribers just won't see the events.
    pub async fn ensure_outbound_pipe(
        &self,
        agent_instance_hierarchy: &str,
        pipes_root: &Path,
        handle: &Handle,
    ) -> broadcast::Sender<SubscribeEvent> {
        // Fast path — already wired.
        if let Some(existing) = self.inner.outbound_senders.get(agent_instance_hierarchy) {
            return existing.clone();
        }

        let address = match events_address_for_agent(pipes_root, agent_instance_hierarchy) {
            Ok(a) => a,
            Err(e) => {
                emit_error(handle, format!("outbound pipe addr for {agent_instance_hierarchy:?}: {e}")).await;
                let (tx, _) = broadcast::channel(OUTBOUND_BROADCAST_CAPACITY);
                return self.install_outbound_sender(agent_instance_hierarchy, tx);
            }
        };

        if let Some(parent) = address.fs_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                emit_error(
                    handle,
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
        // Atomic claim with stale-recovery — see [`bind_or_busy`].
        // The inbound socket is the canonical admission gate, so a
        // `SlotTaken` here in steady state would imply the matching
        // inbound bind had already failed and the process would be
        // exiting. Defensively, we still treat the outbound `Live`
        // case as a degraded condition (return the soft sender) so
        // a rare race doesn't crash the otherwise-healthy stream.
        let listener = match bind_or_busy(&address).await {
            BindOutcome::Bound(l) => l,
            BindOutcome::SlotTaken => {
                emit_error(
                    handle,
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
                emit_error(
                    handle,
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
        let task_handle = handle.clone();
        tokio::spawn(async move {
            run_outbound_listener(listener, task_agent_instance_hierarchy, task_tx, task_handle, cancel_rx).await;
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

    /// Look up the outbound broadcast sender for `agent_instance_hierarchy` if one
    /// has been ensured. Returns `None` for unknown ids.
    pub fn outbound_sender(&self, agent_instance_hierarchy: &str) -> Option<broadcast::Sender<SubscribeEvent>> {
        self.inner
            .outbound_senders
            .get(agent_instance_hierarchy)
            .map(|entry| entry.clone())
    }

    /// Cancel every inbound pipe listener. Reader tasks wake from
    /// their `tokio::select!`, drop their listeners (which unlinks
    /// the filesystem entry), and return. Already-accepted
    /// connection handlers keep running until their peer closes —
    /// that's by design, so the API server can finish flushing any
    /// in-flight notifications before the writer's `notif_rx` sees
    /// every `notif_tx` clone drop and finally closes.
    pub fn shutdown_inbound(&self) {
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

    /// Broadcast [`SubscribeEvent::StreamEnd`] to every outbound
    /// sender currently tracked. Called by the writer task right
    /// after `finalize` returns. Senders stay alive after this call
    /// — [`Self::shutdown_outbound`] is what tears them down.
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

    /// Cancel every outbound listener and drop the per-agent
    /// broadcast senders. Subscriber receivers then see
    /// `RecvError::Closed` (which the per-connection task treats as
    /// "stream done") and the listener tasks unlink `events.sock`.
    /// Run AFTER the writer's `finalize` + `broadcast_stream_end` so
    /// active subscribers see the terminator before disconnect.
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

async fn run_listener(
    listener: Listener,
    agent_instance_hierarchy: String,
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
                        let agent_instance_hierarchy = agent_instance_hierarchy.clone();
                        let response_id = response_id.clone();
                        let handle = handle.clone();
                        tokio::spawn(handle_connection(
                            conn, agent_instance_hierarchy, response_id, notifier, notif_tx, handle,
                        ));
                    }
                    Err(e) => {
                        emit_error(
                            &handle,
                            format!("pipe accept for {agent_instance_hierarchy:?}: {e}"),
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
    agent_instance_hierarchy: String,
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
                emit_error(&handle, format!("pipe read for {agent_instance_hierarchy:?}: {e}")).await;
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
                    "pipe line for {agent_instance_hierarchy:?} is not a valid RichContent JSON: {e}; line: {}",
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
        let _ = notif_tx.send((agent_instance_hierarchy.clone(), response_id.clone(), content.clone()));
        let params = AgentCompletionNotifyParams {
            response_id: response_id.clone(),
            content,
        };
        let ack = match notifier.notify(params).await {
            Ok(()) => PipeAck::Ok,
            Err(e) => {
                let msg = format!("notify dispatch for {agent_instance_hierarchy:?}: {e}");
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

/// Listener loop for an outbound `events.sock`. On each accepted
/// connection, spawns [`handle_outbound_connection`] with a fresh
/// `broadcast::Receiver`. The subscriber only sees events that the
/// writer broadcasts AFTER its connect — exactly the post-connect
/// guarantee the subscribe algorithm relies on for invariant 1.
async fn run_outbound_listener(
    listener: Listener,
    agent_instance_hierarchy: String,
    sender: broadcast::Sender<SubscribeEvent>,
    handle: Handle,
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
                        let handle = handle.clone();
                        tokio::spawn(handle_outbound_connection(conn, agent_instance_hierarchy, rx, handle));
                    }
                    Err(e) => {
                        emit_error(
                            &handle,
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

/// Per-connection task: relay every [`SubscribeEvent`] from the
/// broadcast receiver as one NDJSON line on the socket. Exits when:
///   1. The receiver returns `Closed` — writer task is gone; treat as
///      "stream ended already" and just drop the connection.
///   2. The receiver returns `Lagged` — subscriber didn't keep up
///      past the broadcast buffer. Close the connection so the
///      client can reconnect and restart from a clean watermark.
///   3. A `StreamEnd` event lands — emit it, flush, then close.
///   4. The write fails (broken pipe — client disconnected).
async fn handle_outbound_connection(
    conn: interprocess::local_socket::tokio::Stream,
    agent_instance_hierarchy: String,
    mut rx: broadcast::Receiver<SubscribeEvent>,
    handle: Handle,
) {
    let (_read_half, mut write_half) = conn.split();
    loop {
        let event = match rx.recv().await {
            Ok(ev) => ev,
            Err(broadcast::error::RecvError::Closed) => return,
            Err(broadcast::error::RecvError::Lagged(_)) => {
                // Subscriber fell behind. Closing the connection
                // forces a clean reconnect; the subscribe algorithm
                // treats "pipe disappeared" as "drain again then
                // recheck listener," which is the right recovery.
                return;
            }
        };
        let is_end = matches!(event, SubscribeEvent::StreamEnd);
        let line = match serde_json::to_string(&event) {
            Ok(s) => s,
            Err(e) => {
                emit_error(
                    &handle,
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

async fn emit_error(handle: &Handle, message: String) {
    let out = Output::Error(Error {
        r#type: objectiveai_sdk::cli::output::ErrorType::Error,
        level: Level::Warn,
        fatal: false,
        message: serde_json::Value::String(message),
        agent_instance_hierarchy: None,
    });
    out.emit(handle).await;
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
