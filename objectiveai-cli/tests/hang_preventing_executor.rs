//! `HangPreventingBinaryCommandExecutor` — a wrapping
//! [`CommandExecutor`] that aborts the wrapped cli child when its
//! `the test's state dir` goes quiet for [`HANG_TIMEOUT`]. Drop-in for
//! integration tests so a stuck cli child doesn't wedge nextest's
//! per-binary slot indefinitely.
//!
//! ## Activity definition
//!
//! "Activity" is any `notify` event on `the test's state dir` (recursive).
//! Recursive on Windows + Linux + macOS via `notify::recommended_watcher`.
//!
//! ## Timeout shape
//!
//! Conceptually mirrors `objectiveai-cli/src/command/agents/instances/read/subscribe.rs` —
//! a long-lived task races a [`tokio::time::sleep`] against an event
//! stream. Every event resets the sleep clock; a fired sleep aborts
//! the work. We replace the per-agent unix-socket event stream with a
//! `notify` watcher that produces generic filesystem events on the
//! whole `the test's state dir`.
//!
//! ## Kill semantics
//!
//! When the watchdog fires it drops the inner [`BinaryExecutor`]
//! stream. The wrapped `BinaryExecutor` is constructed with
//! `kill_on_drop(true)` so dropping its stream sends a kill signal to
//! the cli child via `tokio::process::Child`'s `Drop`. On Unix that
//! reaps everything in the child's process group; on Windows only the
//! direct child is killed (grandchildren orphan — a known limit of
//! `tokio::process::Command::kill_on_drop`).

use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use futures::{Stream, StreamExt};
use notify::{RecursiveMode, Watcher};
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use objectiveai_sdk::cli::command::{
    AgentArguments, CommandExecutor, CommandRequest, CommandResponse,
};
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Hardcoded inactivity threshold. If `the test's state dir` sees no
/// filesystem activity for this duration, the wrapped cli child is
/// terminated. Bumped to a constant rather than a constructor
/// parameter for v1 — the value rarely needs to change per call.
pub const HANG_TIMEOUT: Duration = Duration::from_secs(60);

/// Errors surfaced by [`HangPreventingBinaryCommandExecutor`]. Either
/// the wrapped [`BinaryExecutor`]'s own error pass-through, or our
/// own `HangTimeout` variant produced when the watchdog fires.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Inner(#[from] objectiveai_sdk::cli::command::binary::Error),
    #[error(
        "cli child went silent on the test's state dir ({state_dir}) for {elapsed:?} — \
         hang-preventing watchdog killed the child"
    )]
    HangTimeout {
        elapsed: Duration,
        state_dir: PathBuf,
    },
    /// Failed to construct or attach the `notify` filesystem watcher.
    /// Surfaces as a test-side error rather than panicking so the
    /// test reports cleanly.
    #[error("hang-prevention fs watcher setup failed for {state_dir}: {source}")]
    WatcherSetup {
        state_dir: PathBuf,
        #[source]
        source: notify::Error,
    },
}

/// Wraps a [`BinaryExecutor`] with a `the test's state dir` inactivity
/// watchdog. Construct via [`Self::new`]; the wrapped executor is
/// internally forced to `kill_on_drop(true)`.
pub struct HangPreventingBinaryCommandExecutor {
    inner: BinaryExecutor,
    state_dir: PathBuf,
}

impl HangPreventingBinaryCommandExecutor {
    /// Wrap an inner [`BinaryExecutor`]. The inner is forced to
    /// `kill_on_drop(true)` so dropping the inner stream tears the
    /// cli child down when the watchdog fires.
    pub fn new(inner: BinaryExecutor, state_dir: PathBuf) -> Self {
        Self {
            inner: inner.kill_on_drop(true),
            state_dir,
        }
    }

    /// Delegate to the inner [`BinaryExecutor`]'s `.env(...)` builder
    /// so call sites that need to stamp an extra env var on every
    /// spawned child (e.g. `OAI_TEST_MCP_PID_FILE`) can still chain.
    pub fn env(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.inner = self.inner.env(key, value);
        self
    }
}

impl CommandExecutor for HangPreventingBinaryCommandExecutor {
    type Error = Error;
    type Stream<T>
        = Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>
    where
        T: Send + 'static;

    async fn execute<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<Self::Stream<T>, Self::Error>
    where
        R: CommandRequest + Send,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let inner_stream: Pin<
            Box<
                dyn Stream<
                        Item = Result<
                            T,
                            objectiveai_sdk::cli::command::binary::Error,
                        >,
                    > + Send,
            >,
        > = self.inner.execute::<R, T>(request, agent_arguments).await?;

        let state_dir = self.state_dir.clone();
        let (out_tx, out_rx) = mpsc::channel::<Result<T, Error>>(16);
        let (notify_tx, notify_rx) =
            mpsc::unbounded_channel::<notify::Result<notify::Event>>();

        let mut watcher = notify::recommended_watcher(
            move |res: notify::Result<notify::Event>| {
                let _ = notify_tx.send(res);
            },
        )
        .map_err(|e| Error::WatcherSetup {
            state_dir: state_dir.clone(),
            source: e,
        })?;
        watcher
            .watch(&state_dir, RecursiveMode::Recursive)
            .map_err(|e| Error::WatcherSetup {
                state_dir: state_dir.clone(),
                source: e,
            })?;

        tokio::spawn(watchdog_task(
            inner_stream,
            out_tx,
            notify_rx,
            state_dir,
            watcher,
        ));

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(out_rx)))
    }

    async fn execute_one<R, T>(
        &self,
        request: R,
        agent_arguments: Option<&AgentArguments>,
    ) -> Result<T, Self::Error>
    where
        R: CommandRequest + Send,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream = self.execute::<R, T>(request, agent_arguments).await?;
        match stream.next().await {
            Some(item) => item,
            None => Err(Error::Inner(
                objectiveai_sdk::cli::command::binary::Error::Empty,
            )),
        }
    }
}

/// The long-lived task that wraps the inner stream and races it
/// against the inactivity timer. Owns the `notify` watcher to keep it
/// alive (its `Drop` unregisters the watch).
async fn watchdog_task<T>(
    mut inner_stream: Pin<
        Box<
            dyn Stream<
                    Item = Result<T, objectiveai_sdk::cli::command::binary::Error>,
                > + Send,
        >,
    >,
    out_tx: mpsc::Sender<Result<T, Error>>,
    mut notify_rx: mpsc::UnboundedReceiver<notify::Result<notify::Event>>,
    state_dir: PathBuf,
    _watcher: notify::RecommendedWatcher,
) where
    T: Send + 'static,
{
    let started = Instant::now();
    let mut sleeper = Box::pin(tokio::time::sleep(HANG_TIMEOUT));

    loop {
        tokio::select! {
            // Inactivity timer fired → terminate the cli child via
            // dropping `inner_stream` (kill_on_drop=true) and report
            // the hang to the consumer.
            _ = &mut sleeper => {
                let _ = out_tx
                    .send(Err(Error::HangTimeout {
                        elapsed: started.elapsed(),
                        state_dir,
                    }))
                    .await;
                return;
            }
            // Cli emitted an item. Forward it, reset the inactivity
            // timer (a yielded item counts as activity just like a FS
            // event would — otherwise a cli that streams chunks
            // straight through stdout without touching the on-disk
            // logs gets watchdog'd despite being fully alive), and
            // keep going. If the consumer dropped its receiver, also
            // stop.
            next = inner_stream.next() => {
                match next {
                    Some(item) => {
                        let mapped: Result<T, Error> = item.map_err(Error::Inner);
                        if out_tx.send(mapped).await.is_err() {
                            return;
                        }
                        sleeper
                            .as_mut()
                            .reset(Instant::now() + HANG_TIMEOUT);
                    }
                    None => return,
                }
            }
            // Filesystem event on the test's state dir → reset the timer.
            // Errored events still count as "activity" (the inotify
            // descriptor itself doing something).
            Some(_event) = notify_rx.recv() => {
                sleeper
                    .as_mut()
                    .reset(Instant::now() + HANG_TIMEOUT);
            }
        }
    }
}
