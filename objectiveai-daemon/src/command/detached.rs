//! In-process replacement for the former detached self-reexec.
//!
//! Before the single-process refactor, `agents spawn` / `agents
//! message` (wake) / `agents queue deliver` / `functions execute`
//! ran their real streaming work by re-execing THIS binary as a
//! detached orphan subprocess (`BinaryExecutor::detach(true)`),
//! reading only the first `Id` off its stdout, and letting the orphan
//! finish. The subprocess existed purely so the work could outlive the
//! short `/execute` request that only needed the `Id` — a `/execute`
//! run is bound to its WebSocket and is cancelled when the socket
//! closes (`websockets::daemon_execute`).
//!
//! [`spawn_detached`] reproduces that exactly with a `tokio::spawn`
//! task instead of a subprocess: the task re-enters the daemon through
//! [`crate::run`] (the same front door `/execute` uses), so the work
//! rides the full executor stack — tee, transform, token, and TIMEOUT
//! adapters — identically to the subprocess's own `crate::run`. The
//! task owns the run stream, so a client disconnect can never cancel
//! it; dropping the returned `JoinHandle` detaches it, and tokio reaps
//! it on completion. The per-run [`crate::websockets::agent_registry`]
//! (and therefore the agent's lock family) lives INSIDE that stream, so
//! the locks release at true agent-completion — the same lifetime the
//! orphan process gave them.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::RunStream;
use crate::context::Context;
use crate::error::Error;

/// Run `child_request` as a detached in-process daemon task, surfacing
/// items to the returned stream under the control of `forward`:
/// - `None` = skip (don't surface this item);
/// - `Some(false)` = surface it, keep forwarding;
/// - `Some(true)` = surface it, then detach — the returned stream ends
///   and the task drains the rest silently to completion.
///
/// The caller passes a clone of its (per-request) [`Context`]: this
/// keeps the shared `agent_locks` gate and the db/api/python pools that
/// in-process work REQUIRES, while the identity is reset to the
/// daemon's scrubbed default ([`Context::reset_identity`]) so the task
/// runs exactly as the orphan subprocess did (which inherited the
/// daemon process env, not the `/execute` identity override).
///
/// `T` is the leaf response type each site expects; items are decoded
/// into it the same way `BinaryExecutor` decoded the child's stdout
/// JSONL — a serialize + `from_value` round-trip through the untagged
/// top-level shapes.
pub fn spawn_detached<R, T>(
    ctx: Context,
    child_request: R,
    forward: impl Fn(&T) -> Option<bool> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>
where
    R: serde::Serialize + Send + 'static,
    T: serde::de::DeserializeOwned + Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<T, Error>>();
    tokio::spawn(async move {
        let mut ctx = ctx;
        ctx.reset_identity();

        // Feed the leaf request through the top-level `--request` front
        // door — exactly how `BinaryExecutor` fed the former subprocess.
        // The untagged top-level `Request` absorbs the leaf JSON on
        // parse, so no explicit wrapping is needed.
        let json = match serde_json::to_string(&child_request) {
            Ok(json) => json,
            Err(e) => {
                let _ = tx.send(Err(Error::Instance(format!(
                    "serialize detached request: {e}"
                ))));
                return;
            }
        };
        let args = vec!["objectiveai".to_string(), "--request".to_string(), json];
        match crate::run(args, Some(ctx)).await {
            Ok(RunStream::Execute(stream)) => forward_then_drain(stream, tx, forward).await,
            Ok(RunStream::ExecuteTransform(stream)) => {
                forward_then_drain(stream, tx, forward).await
            }
            Err(e) => {
                let _ = tx.send(Err(e));
            }
        }
    });
    Box::pin(UnboundedReceiverStream::new(rx))
}

/// Drain `stream` to completion, decoding each item into `T` and
/// applying `forward` (see [`spawn_detached`]). Once `forward` returns
/// `Some(true)` — or an item fails to decode / arrives as `Err` — the
/// sender is dropped (ending the caller's stream) but draining
/// continues silently: a gone caller must NEVER truncate the run, which
/// is the orphan property the detached subprocess provided.
async fn forward_then_drain<S, I, T>(
    mut stream: S,
    tx: tokio::sync::mpsc::UnboundedSender<Result<T, Error>>,
    forward: impl Fn(&T) -> Option<bool>,
) where
    S: Stream<Item = Result<I, Error>> + Unpin,
    I: serde::Serialize,
    T: serde::de::DeserializeOwned,
{
    let mut tx = Some(tx);
    while let Some(item) = stream.next().await {
        // Past the detach point: keep the run alive, surface nothing.
        let Some(sender) = tx.as_ref() else {
            continue;
        };
        match item {
            Ok(raw) => {
                // Re-decode into the leaf type the same way the former
                // subprocess's stdout JSONL was decoded.
                match serde_json::to_value(&raw).and_then(serde_json::from_value::<T>) {
                    Ok(value) => match forward(&value) {
                        None => {}
                        Some(false) => {
                            let _ = sender.send(Ok(value));
                        }
                        Some(true) => {
                            let _ = sender.send(Ok(value));
                            tx = None;
                        }
                    },
                    Err(e) => {
                        let _ = sender.send(Err(Error::Instance(format!(
                            "decode detached item: {e}"
                        ))));
                        tx = None;
                    }
                }
            }
            Err(e) => {
                let _ = sender.send(Err(e));
                tx = None;
            }
        }
    }
}
