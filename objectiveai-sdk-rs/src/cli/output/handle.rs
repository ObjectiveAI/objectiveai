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

use serde::Serialize;
use tokio::process::ChildStdin;
use tokio::sync::{Mutex, mpsc};

use super::{Notification, Output};

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
    /// Push each emitted `Output<T>` (reserialized as `Output<Value>`
    /// for a uniform storage type) into a shared vector.
    Collect(Arc<Mutex<Vec<Output<serde_json::Value>>>>),
    /// Forward each emitted `Output<T>` through an mpsc channel
    /// (reserialized as `Output<Value>` to match `Collect`'s storage
    /// type). The receiver loops on `rx.recv()` until the cli's
    /// `run()` completes and the `Handle` is dropped — at which point
    /// the sender side closes and the receiver's loop exits.
    Stream(mpsc::UnboundedSender<Output<serde_json::Value>>),
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

    /// Emit `output` to this destination. Panics on write failure to
    /// match `println!` semantics.
    pub async fn emit<T: Serialize>(&self, output: &Output<T>) {
        // Single Value round-trip when we have an agent_id to stamp.
        // Only `Notification` and `Error` get tagged; `Begin` / `End`
        // are unit-shaped framing markers and stay as-is.
        let json = if let Some(id) = self.agent_id.as_deref() {
            let mut v = serde_json::to_value(output)
                .expect("Output<T> serializes when T: Serialize");
            if matches!(output, Output::Notification(_) | Output::Error(_)) {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert(
                        "agent_id".to_string(),
                        serde_json::Value::String(id.to_string()),
                    );
                }
            }
            serde_json::to_string(&v)
                .expect("agent-id-stamped Output<T> reserializes")
        } else {
            serde_json::to_string(output)
                .expect("Output<T> serializes when T: Serialize")
        };
        match &self.destination {
            HandleDestination::Stdout => {
                println!("{json}");
                if matches!(output, Output::Error(e) if e.fatal) {
                    eprintln!("{json}");
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
                vec.lock().await.push(self.rebuild_as_value(output));
            }
            HandleDestination::Stream(tx) => {
                // Best-effort send: if the receiver dropped (consumer
                // gone), just drop the message — same semantics as
                // Stdout where a closed pipe would crash.
                let _ = tx.send(self.rebuild_as_value(output));
            }
        }
    }

    /// Rebuild `Output<T>` as `Output<Value>` variant-wise, avoiding a
    /// full serialize→deserialize roundtrip. Used by `Collect` and
    /// `Stream`. Picks up the handle's `agent_id` on `Notification`
    /// and `Error` so in-memory consumers see the same stamped shape
    /// the wire output carries.
    fn rebuild_as_value<T: Serialize>(
        &self,
        output: &Output<T>,
    ) -> Output<serde_json::Value> {
        match output {
            Output::Error(e) => {
                let mut e = e.clone();
                if e.agent_id.is_none() {
                    e.agent_id = self.agent_id.clone();
                }
                Output::Error(e)
            }
            Output::Notification(n) => Output::Notification(Notification {
                value: serde_json::to_value(&n.value)
                    .expect("T serializes when T: Serialize"),
                agent_id: n.agent_id.clone().or_else(|| self.agent_id.clone()),
            }),
            Output::Begin => Output::Begin,
            Output::End => Output::End,
        }
    }
}
