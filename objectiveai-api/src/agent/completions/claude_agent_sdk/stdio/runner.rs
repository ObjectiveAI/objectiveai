//! Long-lived runner subprocess + per-request dispatcher.
//!
//! One [`Runner`] owns one Python runner subprocess. The subprocess is
//! spawned lazily (via [`tokio::sync::OnceCell`] on the parent
//! `super::super::Client`) and then reused for every subsequent
//! `agent_completions::create` call against that client. Concurrency
//! comes from the runner's own asyncio multiplexer — it accepts N
//! concurrent `run` requests over a single (stdin, stdout, stderr)
//! triple. The FIFO `query_limit` is enforced on the Rust side via a
//! [`tokio::sync::Semaphore`] on `Runner`; surplus requests wait for a
//! permit before their `run` line is sent, so the Python runner sees
//! only what we let through.
//!
//! ## Architecture
//!
//! ```text
//!   Client::create_streaming  ──┐
//!                                │  register(id) → Receiver<Update>
//!                                │
//!                                │  send Run ─→ stdin
//!                                ▼
//!   ┌──────────────  Arc<Runner>  ──────────────┐
//!   │  Mutex<ChildStdin>                         │
//!   │  Mutex<HashMap<id, mpsc::Sender<Update>>>  │
//!   │                                            │
//!   │   stdout reader task ◀── stdout (NDJSON)   │
//!   │   stderr reader task ◀── stderr (NDJSON)   │
//!   └────────────────────────────────────────────┘
//!                                │  for each line:
//!                                │    deserialize → look up sender by id
//!                                │                  → forward Update
//!                                ▼
//!   Per-request consumer pulls Updates → maps to StreamItem<State>
//! ```
//!
//! Crash handling: when either reader sees EOF, all in-flight senders
//! are dropped (their consumers see channel closure). Subsequent
//! `register`/`send_*` calls fail with [`RunnerError::Closed`]. We do
//! not auto-restart here — the caller can drop the parent client's
//! `OnceCell` and let the next request spawn a fresh runner if/when
//! we add that policy.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio::task::JoinHandle;

use super::super::sdk_message::SDKMessage;
use super::{
    RunParams, RunnerError, RunnerStream, RunnerUpdate, StdioError, StdioInput,
    StdioOutput,
};

type Registry = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<RunnerUpdate>>>>;

/// One long-lived Claude Agent SDK runner subprocess. Created lazily
/// by `super::super::Client` via `tokio::sync::OnceCell`.
pub struct Runner {
    /// stdin write half. Held under a mutex so concurrent `send_run`
    /// calls can't interleave bytes mid-line.
    stdin: Mutex<ChildStdin>,

    /// id → sender for the per-request channel. Cleared when a
    /// request's terminal `end` is forwarded; entries are also
    /// dropped wholesale when either reader sees EOF (signals close).
    registry: Registry,

    /// Once true, no further sends or registrations are accepted.
    /// Set by the reader tasks when they detect EOF on either pipe.
    closed: Arc<std::sync::atomic::AtomicBool>,

    /// FIFO concurrency cap on in-flight requests. `create_stream`
    /// acquires a permit *before* sending `run` to stdin, and the
    /// returned [`RunnerStream`] holds the permit for its lifetime.
    /// `tokio::sync::Semaphore` is FIFO (waiters live in a queue,
    /// `add_permits` wakes the oldest), so accepted-order equals
    /// run-order. Queued requests never touch the runner subprocess.
    semaphore: Arc<Semaphore>,

    /// Reader tasks are kept alive by their JoinHandles being held
    /// here. Aborted on drop.
    _stdout_task: JoinHandle<()>,
    _stderr_task: JoinHandle<()>,

    /// The runner subprocess handle. Held here so the OS process
    /// stays alive for the lifetime of the Runner. Dropping this
    /// (when Runner drops) sends SIGKILL via `kill_on_drop(true)`
    /// — last in struct-field order so `stdin` (declared first)
    /// has already been dropped, giving the Python process a
    /// chance to see EOF on stdin and drain naturally before the
    /// kill fires. **Do not drop this field early** — doing so
    /// kills the subprocess immediately and every subsequent
    /// request fails with `runner subprocess has exited`.
    _child: Child,
}

impl Runner {
    /// Spawn the runner subprocess, set up the stdout/stderr reader
    /// tasks, and return a handle. The Python runner has no built-in
    /// concurrency cap; the FIFO `query_limit` is enforced entirely
    /// here by the `tokio::sync::Semaphore` stored on `Self`.
    pub async fn spawn(binary: &str, query_limit: u64) -> Result<Self, RunnerError> {
        let mut cmd = Command::new(binary);
        objectiveai_sdk::process::no_window(&mut cmd);
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        cmd.env_remove("CLAUDECODE");
        // Leash the runner to this process via the OS-level
        // subprocess-reaper, so it dies if the api goes away by ANY means
        // — force-kill (`kill -9` / `TerminateProcess`), crash, or normal
        // exit — not just an orderly `Drop`. (The reaper also sets
        // `kill_on_drop(true)` itself as the in-process fast path.)
        let mut child = objectiveai_sdk::subprocess_reaper::spawn(&mut cmd)
            .map_err(|e| RunnerError::Spawn(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| RunnerError::Spawn("stdin not piped".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RunnerError::Spawn("stdout not piped".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| RunnerError::Spawn("stderr not piped".into()))?;

        let registry: Registry = Arc::new(Mutex::new(HashMap::new()));
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let semaphore = Arc::new(Semaphore::new(query_limit as usize));

        // Stdout reader task — fully parses each line as
        // StdioOutput<SDKMessage> and forwards to the registered
        // sender for that id. Lines for unknown ids are dropped
        // (most likely the consumer hung up after the runner had
        // already buffered some output).
        let stdout_task = {
            let registry = registry.clone();
            let closed = closed.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    Self::dispatch_stdout(trimmed, &registry).await;
                }
                // EOF or read error — runner is gone.
                Self::close_all(&registry, &closed).await;
            })
        };

        // Stderr reader task — typed as StdioError. `Diag` lines route
        // to the per-id channel; `Fatal` lines fan out to every
        // in-flight request.
        let stderr_task = {
            let registry = registry.clone();
            let closed = closed.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    Self::dispatch_stderr(trimmed, &registry).await;
                }
                // EOF or read error.
                Self::close_all(&registry, &closed).await;
            })
        };

        // Keep `child` alive on the Runner. Field-drop order
        // (stdin first, _child last) means: when Runner drops,
        // stdin closes → the Python runner sees EOF and starts to
        // drain → moments later `_child` drops and `kill_on_drop`
        // fires SIGKILL as a fallback in case drain hasn't
        // finished. Dropping `child` here at spawn-time would kill
        // the subprocess immediately (kill_on_drop is unconditional
        // on drop, not just on panic) and every subsequent request
        // would fail with `runner subprocess has exited`.

        Ok(Self {
            stdin: Mutex::new(stdin),
            registry,
            closed,
            semaphore,
            _stdout_task: stdout_task,
            _stderr_task: stderr_task,
            _child: child,
        })
    }

    /// Start one in-flight request: acquire a FIFO permit (the
    /// concurrency cap), register an id, send the `run`, and return a
    /// [`RunnerStream`] of updates for it. The returned stream holds
    /// the permit for its lifetime, so the cap is released on drop /
    /// terminal update. Dropping the stream unregisters the id but
    /// does **not** cancel the in-flight SDK work — the runner runs
    /// the request to natural completion regardless. See
    /// [`RunnerStream`] for the rationale.
    ///
    /// `create_stream` borrows `self` through an `Arc` so the stream
    /// can keep the runner alive for its drop handler. The expected
    /// caller already holds an `Arc<Runner>` via the parent client's
    /// `OnceCell`.
    pub async fn create_stream<'a>(
        self: &Arc<Self>,
        id: String,
        params: RunParams<'a>,
    ) -> Result<RunnerStream, RunnerError> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Runner semaphore is never closed");
        let rx = self.register(id.clone()).await?;
        if let Err(e) = self.send_run(&id, params).await {
            // The runner never accepted the id, so there's nothing
            // to cancel — just clean up the registry entry. Permit
            // is released when it drops at the end of this scope.
            self.unregister(&id).await;
            return Err(e);
        }
        Ok(RunnerStream::new(rx, self.clone(), id, permit))
    }

    /// Register a new request `id`. Internal — `create_stream` is the
    /// public entry point.
    async fn register(
        &self,
        id: String,
    ) -> Result<mpsc::UnboundedReceiver<RunnerUpdate>, RunnerError> {
        if self.closed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(RunnerError::Closed);
        }
        let (tx, rx) = mpsc::unbounded_channel();
        let mut reg = self.registry.lock().await;
        if reg.contains_key(&id) {
            return Err(RunnerError::DuplicateId(id));
        }
        reg.insert(id, tx);
        Ok(rx)
    }

    /// Drop a request's registry entry. Idempotent. Internal —
    /// called by `create_stream` on send-failure and by
    /// `RunnerStream::drop`.
    pub(super) async fn unregister(&self, id: &str) {
        let mut reg = self.registry.lock().await;
        reg.remove(id);
    }

    /// Send a `run` request to the runner's stdin. Internal — used
    /// by `create_stream` after `register` succeeds.
    async fn send_run<'a>(
        &self,
        id: &'a str,
        params: RunParams<'a>,
    ) -> Result<(), RunnerError> {
        if self.closed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(RunnerError::Closed);
        }
        let request = StdioInput::Run { id, params };
        self.write_line(&request).await
    }


    /// Serialize one inbound message and write it to stdin under the
    /// stdin lock so concurrent senders can't interleave bytes.
    async fn write_line(&self, request: &StdioInput<'_>) -> Result<(), RunnerError> {
        let mut line = serde_json::to_vec(request)?;
        line.push(b'\n');
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(&line)
            .await
            .map_err(|e| RunnerError::Write(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| RunnerError::Write(e.to_string()))?;
        Ok(())
    }

    /// Process one stdout line — full-deserialize and route the
    /// resulting [`RunnerUpdate`] to the per-id channel. Malformed
    /// lines are dropped silently; they'd be a runner bug, and we
    /// keep going so other in-flight requests keep working.
    async fn dispatch_stdout(line: &str, registry: &Registry) {
        let parsed: StdioOutput<SDKMessage> = match serde_json::from_str(line) {
            Ok(p) => p,
            Err(_) => return,
        };
        match parsed {
            StdioOutput::Event { id, event } => {
                Self::send_to(registry, &id, RunnerUpdate::Event(event)).await;
            }
            StdioOutput::End { id, status } => {
                // Forward terminal status, then drop the registry
                // entry — no more updates will come for this id.
                let mut reg = registry.lock().await;
                if let Some(tx) = reg.remove(&id) {
                    let _ = tx.send(RunnerUpdate::End(status));
                }
            }
        }
    }

    /// Process one stderr line — route `diag` to the per-id channel,
    /// fan `fatal` out to every in-flight request.
    async fn dispatch_stderr(line: &str, registry: &Registry) {
        let parsed: StdioError = match serde_json::from_str(line) {
            Ok(p) => p,
            Err(_) => return,
        };
        match parsed {
            StdioError::Diag {
                id,
                level,
                message,
            } => {
                Self::send_to(
                    registry,
                    &id,
                    RunnerUpdate::Diag { level, message },
                )
                .await;
            }
            StdioError::Fatal { message } => {
                // Fan-out: every in-flight request gets the fatal,
                // and the registry is cleared so no one sees more
                // updates.
                let mut reg = registry.lock().await;
                let entries: Vec<_> = reg.drain().collect();
                drop(reg);
                for (_, tx) in entries {
                    let _ = tx.send(RunnerUpdate::Fatal(message.clone()));
                }
            }
        }
    }

    async fn send_to(registry: &Registry, id: &str, update: RunnerUpdate) {
        let reg = registry.lock().await;
        if let Some(tx) = reg.get(id) {
            // Send-failure means the receiver was dropped (consumer
            // hung up) — that's fine, just discard.
            let _ = tx.send(update);
        }
    }

    /// Mark the runner as closed and notify every in-flight request
    /// that the subprocess is gone. Idempotent.
    async fn close_all(registry: &Registry, closed: &Arc<std::sync::atomic::AtomicBool>) {
        if closed.swap(true, std::sync::atomic::Ordering::AcqRel) {
            return; // already closed
        }
        let mut reg = registry.lock().await;
        let entries: Vec<_> = reg.drain().collect();
        drop(reg);
        for (_, tx) in entries {
            let _ = tx.send(RunnerUpdate::RunnerExited);
        }
    }
}
