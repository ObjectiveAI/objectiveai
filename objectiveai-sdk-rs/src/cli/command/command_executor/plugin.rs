use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use dashmap::DashMap;
use futures::{Stream, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, mpsc};

use crate::cli::command::{
    AgentArguments, CommandExecutor, CommandRequest,
    CommandResponse as CommandResponseTrait,
};
use crate::cli::plugins::{Command, CommandType, Output};

/// Demultiplex many in-flight `CommandRequest` calls over a plugin's
/// stdin/stdout. Each `execute` mints a fresh id, emits a
/// `Output::Command(Command { id, command })` line on the plugin's stdout,
/// and returns a stream that yields whatever the overlord writes back
/// to the plugin's stdin under the same id.
///
/// Only one instance per process — the constructor consumes the global
/// `tokio::io::stdin()` / `stdout()` handles. The struct is [`Clone`]
/// so callers that need a second handle can share without an outer
/// `Arc`: every field is already behind `Arc`, including `counter`, so
/// clones share the id sequence and pending map.
#[derive(Clone)]
pub struct PluginExecutor {
    stdout: Arc<Mutex<tokio::io::Stdout>>,
    counter: Arc<AtomicU64>,
    pending: Arc<DashMap<String, mpsc::UnboundedSender<serde_json::Value>>>,
    /// `true` while the listener task is still reading stdin. Flipped
    /// to `false` immediately before the listener drops its pending
    /// senders, so `execute()` can re-check after registering its own
    /// sender and bail with `Error::Closed` instead of installing a
    /// channel nothing will ever drain.
    listener_alive: Arc<AtomicBool>,
}

impl Default for PluginExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginExecutor {
    /// Capture the plugin's stdin/stdout and spawn the demuxer task.
    pub fn new() -> Self {
        let pending: Arc<DashMap<String, mpsc::UnboundedSender<serde_json::Value>>> =
            Arc::new(DashMap::new());
        let listener_alive = Arc::new(AtomicBool::new(true));
        Self::spawn_listener(
            tokio::io::stdin(),
            pending.clone(),
            listener_alive.clone(),
        );
        Self {
            stdout: Arc::new(Mutex::new(tokio::io::stdout())),
            counter: Arc::new(AtomicU64::new(0)),
            pending,
            listener_alive,
        }
    }

    fn spawn_listener(
        stdin: tokio::io::Stdin,
        pending: Arc<DashMap<String, mpsc::UnboundedSender<serde_json::Value>>>,
        listener_alive: Arc<AtomicBool>,
    ) {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdin).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let env = match serde_json::from_str::<CommandResponse>(&line) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                match env {
                    CommandResponse::Value { id, value } => {
                        // Host-form stream terminator:
                        // `{id, value:{type:"command_complete",…}}`. Swallow
                        // it — end this id's stream, never forward it on.
                        if value.get("type").and_then(serde_json::Value::as_str)
                            == Some("command_complete")
                        {
                            pending.remove(&id);
                        } else if let Some(sender) = pending.get(&id) {
                            if sender.send(value).is_err() {
                                drop(sender);
                                pending.remove(&id);
                            }
                        }
                    }
                    CommandResponse::Done { id, .. } => {
                        pending.remove(&id);
                    }
                }
            }
            // stdin EOF or read error. Flip the liveness flag BEFORE
            // dropping any senders — `execute()` does an insert-then-
            // re-check, and this ordering guarantees a concurrent
            // `execute()` either sees the flag and removes its own
            // entry, or completes its insert before `clear()` runs and
            // gets drained by it.
            listener_alive.store(false, Ordering::Release);
            pending.clear();
        });
    }
}

/// One line the overlord writes to a plugin's stdin in response to a
/// previously-emitted `Output::Command`.
///
/// Wire shape:
/// - Value: `{"id":"42","value":<JSON>}`
/// - Done:  `{"id":"42","done":true}`
///
/// Two end-of-stream markers are recognized for an id, after which the
/// request's stream ends: the `Done` form above, and the host's
/// `command_complete` form — a `Value` whose JSON is
/// `{"type":"command_complete","exit_code":N}` (handled in the listener,
/// swallowed, never surfaced to the consumer).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
enum CommandResponse {
    /// Listed first so the untagged decoder tries it before `Value` —
    /// the `done` discriminator field is what tells the variants apart.
    Done {
        id: String,
        #[allow(dead_code)]
        done: bool,
    },
    Value {
        id: String,
        value: serde_json::Value,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Stdin closed (clean EOF or read error). The listener task has
    /// exited and no new requests can be served.
    #[error("plugin executor stdin closed")]
    Closed,
    #[error("plugin executor io: {0}")]
    Io(std::io::Error),
    #[error("plugin executor decode line: {0}")]
    Json(serde_json::Error),
    #[error("{0}")]
    Cli(crate::cli::Error),
    /// `execute_one` was called but the stream produced no items.
    #[error("plugin executor stream produced no items")]
    Empty,
}

/// Per-value untagged decode. `Err` first so `cli::Error`'s `type:"error"`
/// constant short-circuits non-error wire shapes; `Ok(T)` is the
/// fallthrough. Mirrors the helper in `binary.rs`.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Line<T> {
    Err(crate::cli::Error),
    Ok(T),
}

impl<T> From<Line<T>> for Result<T, Error> {
    fn from(line: Line<T>) -> Self {
        match line {
            Line::Err(e) => Err(Error::Cli(e)),
            Line::Ok(t) => Ok(t),
        }
    }
}

impl CommandExecutor for PluginExecutor {
    type Error = Error;
    type Stream<T>
        = Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>
    where
        T: Send + 'static;

    async fn execute<R, T>(
        &self,
        request: R,
        // Plugin runs in-process — no subprocess to stamp env on. The
        // bag is accepted for trait-signature symmetry with the
        // binary executor and intentionally ignored.
        _agent_arguments: Option<&AgentArguments>,
    ) -> Result<Self::Stream<T>, Error>
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponseTrait + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let id = self.counter.fetch_add(1, Ordering::Relaxed).to_string();
        let (tx, rx) = mpsc::unbounded_channel::<serde_json::Value>();
        self.pending.insert(id.clone(), tx);

        // Re-check liveness AFTER insert. The listener stores `false`
        // before it calls `pending.clear()`, so any of these happens:
        //   - We see `true`: listener is still running; if it dies
        //     later, its `clear()` will drop our sender and the stream
        //     will end naturally.
        //   - We see `false` and our entry got cleared: remove() is a
        //     no-op, sender is already dropped.
        //   - We see `false` and our entry survived (we inserted after
        //     `clear()` ran): remove() drops the sender ourselves.
        // In every `false` path we bail with `Closed`.
        if !self.listener_alive.load(Ordering::Acquire) {
            self.pending.remove(&id);
            return Err(Error::Closed);
        }

        // Carry the typed request as JSON via the cli's `--request` flag;
        // the host re-enters `run` with this argv. (request -> argv lowering
        // no longer exists.)
        let argv = vec![
            "--request".to_string(),
            serde_json::to_string(&request).map_err(Error::Json)?,
        ];
        let envelope = Output::Command(Command {
            r#type: CommandType::Command,
            id: id.clone(),
            // Carry argv structured — joining into a single string
            // would lose argument boundaries for any value containing
            // whitespace (e.g. `--simple "a b c"`), which the host
            // could not recover.
            command: argv,
        });
        let line = serde_json::to_string(&envelope).expect("Output serializes");

        {
            let mut stdout = self.stdout.lock().await;
            if let Err(e) = stdout.write_all(line.as_bytes()).await {
                self.pending.remove(&id);
                return Err(Error::Io(e));
            }
            if let Err(e) = stdout.write_all(b"\n").await {
                self.pending.remove(&id);
                return Err(Error::Io(e));
            }
            if let Err(e) = stdout.flush().await {
                self.pending.remove(&id);
                return Err(Error::Io(e));
            }
        }

        let pending = self.pending.clone();
        let stream = futures::stream::unfold(
            (rx, id, pending),
            |(mut rx, id, pending)| async move {
                match rx.recv().await {
                    Some(value) => {
                        let item = match serde_json::from_value::<Line<T>>(value) {
                            Ok(line) => line.into(),
                            Err(e) => Err(Error::Json(e)),
                        };
                        Some((item, (rx, id, pending)))
                    }
                    None => {
                        pending.remove(&id);
                        None
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn execute_one<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<T, Error>
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponseTrait + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream = self.execute::<R, T>(request, agent_arguments).await?;
        stream.next().await.ok_or(Error::Empty)?
    }
}
