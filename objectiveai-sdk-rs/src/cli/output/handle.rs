//! Destinations for [`super::Output::emit`].
//!
//! - [`HandleDestination::Stdout`] — this process's stdout (with
//!   fatal-error mirror to stderr).
//! - [`HandleDestination::Stdin`] — a child process's stdin, for
//!   programmatic embedders that spawn a subprocess to consume the
//!   cli's output.
//! - [`HandleDestination::Collect`] — an in-memory shared `Vec`, for
//!   tests or in-process embedders that want the events without a
//!   subprocess.
//! - [`HandleDestination::Stream`] — push each emitted output through
//!   an mpsc channel. Used by hosts that embed the cli in-process and
//!   want each [`Output`] line as a typed value (e.g. the viewer
//!   bridging cli output into Tauri events).
//!
//! `Handle` is a thin pair of (`destination`, `agent_id`). The cli's
//! top-level `run()` stamps `agent_id` once at startup; from then on
//! every emit picks up the same value without any per-call site
//! threading.

use std::sync::Arc;

use tokio::process::ChildStdin;
use tokio::sync::{Mutex, mpsc};

use super::Output;

/// Destination for [`super::Output::emit`].
///
/// `Arc<tokio::sync::Mutex<_>>` on the non-unit variants lets the
/// handle be cloned cheaply across the command tree's call chain and
/// guarantees concurrent emits serialize correctly. `tokio::sync::Mutex`
/// (not `std::sync::Mutex`) because we hold the guard across `.await`
/// boundaries during async writes, which would deadlock with std's.
#[derive(Clone)]
pub enum HandleDestination {
    /// Write each line to this process's stdout; mirror fatal
    /// `Output::Error` lines to stderr.
    Stdout,
    /// Write each line to a child process's stdin.
    Stdin(Arc<Mutex<ChildStdin>>),
    /// Push each emitted [`Output`] into a shared vector. Used by
    /// tests and by in-process embedders that want to observe every
    /// emitted line.
    Collect(Arc<Mutex<Vec<Output>>>),
    /// Forward each emitted [`Output`] through an mpsc channel. The
    /// receiver loops on `rx.recv()` until the cli's `run()` completes
    /// and the `Handle` is dropped — at which point the sender side
    /// closes and the receiver's loop exits.
    Stream(mpsc::UnboundedSender<Output>),
}

impl Default for HandleDestination {
    fn default() -> Self {
        HandleDestination::Stdout
    }
}

/// Output handle: a destination plus an optional `agent_id` that gets
/// stamped on every emitted `Notification` and `Error` line.
///
/// The `agent_id` field is set once by the cli's `run()` from
/// `Config.agent_id` (env `OBJECTIVEAI_AGENT_ID`). All emit sites
/// stay verbatim — `Notification` and `Error` payloads carry their
/// own `agent_id: Option<String>` field that defaults to `None`, and
/// `emit` overwrites it with the handle's value before writing.
#[derive(Clone, Default)]
pub struct Handle {
    pub destination: HandleDestination,
    pub agent_id: Option<String>,
}

impl From<HandleDestination> for Handle {
    fn from(destination: HandleDestination) -> Self {
        Handle { destination, agent_id: None }
    }
}

impl Handle {
    /// Sugar for the most common destination — write to this process's
    /// stdout. Equivalent to `Handle::default()`.
    pub fn stdout() -> Self {
        Handle::default()
    }

    /// Emit `output` to this destination. Best-effort for `Stdout`:
    /// if the parent's read end of our pipe has closed (broken pipe)
    /// we silently swallow the error so a detached background writer
    /// can keep running after the parent exits — matches the
    /// `Stream`/`Collect` destinations' "drop on closed receiver"
    /// behaviour. Non-broken-pipe write errors fall through to a
    /// panic so genuine bugs still surface.
    pub async fn emit(&self, output: &Output) {
        // Single Value round-trip when we have an agent_id to stamp.
        // Both remaining variants (Notification + Error) are
        // object-shaped, so the insert always lands on a real object.
        let json = if let Some(id) = self.agent_id.as_deref() {
            let mut v = serde_json::to_value(output)
                .expect("Output serializes");
            if let Some(obj) = v.as_object_mut() {
                obj.insert(
                    "agent_id".to_string(),
                    serde_json::Value::String(id.to_string()),
                );
            }
            serde_json::to_string(&v)
                .expect("agent-id-stamped Output reserializes")
        } else {
            serde_json::to_string(output)
                .expect("Output serializes")
        };
        match &self.destination {
            HandleDestination::Stdout => {
                use std::io::Write;
                let mut out = std::io::stdout().lock();
                match writeln!(out, "{json}") {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                        // Parent's read end closed. Detached writers
                        // (e.g. cli-stream after its CLI parent
                        // exited on `LogStreamReady`) keep running.
                    }
                    Err(e) => {
                        panic!("emit to stdout failed: {e}");
                    }
                }
                if matches!(output, Output::Error(e) if e.fatal) {
                    let mut err = std::io::stderr().lock();
                    let _ = writeln!(err, "{json}");
                }
            }
            HandleDestination::Stdin(stdin) => {
                use tokio::io::AsyncWriteExt;
                let mut guard = stdin.lock().await;
                guard
                    .write_all(json.as_bytes())
                    .await
                    .expect("emit to child stdin failed");
                guard
                    .write_all(b"\n")
                    .await
                    .expect("emit to child stdin failed");
            }
            HandleDestination::Collect(vec) => {
                vec.lock().await.push(self.stamped(output));
            }
            HandleDestination::Stream(tx) => {
                // Best-effort send: if the receiver dropped (consumer
                // gone), just drop the message — same semantics as
                // Stdout where a closed pipe would crash.
                let _ = tx.send(self.stamped(output));
            }
        }
    }

    /// Clone `output` with the handle's `agent_id` stamped in if the
    /// output didn't already carry one. Used by `Collect` and
    /// `Stream` so in-memory consumers see the same stamped shape
    /// the wire output carries.
    fn stamped(&self, output: &Output) -> Output {
        match output {
            Output::Error(e) => {
                let mut e = e.clone();
                if e.agent_id.is_none() {
                    e.agent_id = self.agent_id.clone();
                }
                Output::Error(e)
            }
            Output::Notification(n) => {
                let mut n = n.clone();
                if n.agent_id.is_none() {
                    n.agent_id = self.agent_id.clone();
                }
                Output::Notification(n)
            }
        }
    }
}

/// Reverse of [`Handle::emit`]'s `agent_id` stamping: walks each
/// newline-delimited line in `text`, tries to parse it as a JSON
/// object, and removes the top-level `"agent_id"` key. Lines that
/// aren't valid JSON pass through unchanged. Preserves the trailing
/// newline if the original had one (the emit envelope always ends
/// with `\n`, so matching that keeps re-stripped bodies stable).
///
/// Used by test-mode response collectors (e.g. the MCP server's
/// `TEST_MODE` strip) and snapshot normalizers to keep the racy
/// `agent_id` counter out of test artefacts. **Never use this on
/// production wire output** — callers depend on the agent_id stamp
/// for cross-process correlation.
pub fn strip_agent_id_lines(text: &str) -> String {
    let mut out: String = text
        .lines()
        .map(|line| {
            // Only rewrite lines that parse as a JSON *object* AND
            // actually contain an `agent_id` top-level key. Other
            // JSON-valid lines (strings, numbers, arrays, or
            // objects without agent_id) pass through verbatim —
            // otherwise we'd re-serialize an indented JSON-string
            // line like `    "object"` (a schema's enum value) as
            // `"object"`, destroying the surrounding pretty-printed
            // indentation that downstream consumers depend on.
            let Ok(serde_json::Value::Object(mut obj)) =
                serde_json::from_str::<serde_json::Value>(line)
            else {
                return line.to_string();
            };
            if obj.remove("agent_id").is_none() {
                return line.to_string();
            }
            serde_json::to_string(&serde_json::Value::Object(obj))
                .unwrap_or_else(|_| line.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}
