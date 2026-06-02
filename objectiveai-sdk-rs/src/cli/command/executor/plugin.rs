use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use futures::Stream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, mpsc};

use crate::cli::command::{CommandExecutor, CommandRequest};
use crate::cli::plugins::{Output, TypedOutput};

/// Demultiplex many in-flight `CommandRequest` calls over a plugin's
/// stdin/stdout. Each `execute` mints a fresh id, emits a
/// `TypedOutput::Command { id, command }` line on the plugin's stdout,
/// and returns a stream that yields whatever the overlord writes back
/// to the plugin's stdin under the same id.
///
/// Only one instance per process — the constructor consumes the global
/// `tokio::io::stdin()` / `stdout()` handles.
pub struct PluginExecutor {
    stdout: Arc<Mutex<tokio::io::Stdout>>,
    counter: AtomicU64,
    pending: Arc<DashMap<String, mpsc::UnboundedSender<serde_json::Value>>>,
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
        Self::spawn_listener(tokio::io::stdin(), pending.clone());
        Self {
            stdout: Arc::new(Mutex::new(tokio::io::stdout())),
            counter: AtomicU64::new(0),
            pending,
        }
    }

    fn spawn_listener(
        stdin: tokio::io::Stdin,
        pending: Arc<DashMap<String, mpsc::UnboundedSender<serde_json::Value>>>,
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
                        if let Some(sender) = pending.get(&id) {
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
            // stdin EOF: drop every pending sender so all in-flight
            // streams observe channel close and terminate.
            pending.clear();
        });
    }
}

/// One line the overlord writes to a plugin's stdin in response to a
/// previously-emitted `TypedOutput::Command`.
///
/// Wire shape:
/// - Value: `{"id":"42","value":<JSON>}`
/// - Done:  `{"id":"42","done":true}`
///
/// `Done` signals end-of-stream for that id from the receiver's
/// perspective — the request's stream ends right after.
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

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Cli(crate::cli::Error),
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

    async fn execute<R, T>(&self, request: R) -> Result<Self::Stream<T>, Error>
    where
        R: CommandRequest + Send,
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let id = self.counter.fetch_add(1, Ordering::Relaxed).to_string();
        let (tx, rx) = mpsc::unbounded_channel::<serde_json::Value>();
        self.pending.insert(id.clone(), tx);

        let argv = request.into_command();
        let envelope = Output::Typed(TypedOutput::Command {
            id: id.clone(),
            command: argv.join(" "),
        });
        let line = serde_json::to_string(&envelope).expect("Output serializes");

        {
            let mut stdout = self.stdout.lock().await;
            stdout.write_all(line.as_bytes()).await.map_err(Error::Io)?;
            stdout.write_all(b"\n").await.map_err(Error::Io)?;
            stdout.flush().await.map_err(Error::Io)?;
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
}
