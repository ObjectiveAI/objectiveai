use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::{mpsc, OwnedSemaphorePermit};

use super::{Runner, RunnerUpdate};

/// Stream of [`RunnerUpdate`]s for a single in-flight request.
///
/// Returned by [`Runner::create_stream`]. Drives one entry in the
/// runner's per-id registry; the stream's [`Drop`] impl handles
/// registry cleanup:
///
/// - If the stream saw a terminal update (any [`RunnerUpdate::End`]
///   variant, [`RunnerUpdate::Fatal`], or [`RunnerUpdate::RunnerExited`]),
///   or the underlying channel closed (poll returned `Ready(None)`),
///   the registry entry has already been removed and `drop` does
///   nothing.
/// - Otherwise, `drop` spawns a best-effort task that unregisters
///   the id so the registry doesn't accumulate dead entries.
///
/// Drop does **not** cancel the in-flight Codex SDK work. Codex's
/// `Thread` doesn't expose a contract for stopping between billing
/// events, so once a request is sent the runner runs it to natural
/// completion. A consumer that walks away mid-stream is still billed
/// for the work in flight.
pub struct RunnerStream {
    rx: mpsc::UnboundedReceiver<RunnerUpdate>,
    runner: Arc<Runner>,
    id: String,
    completed: bool,
    /// FIFO concurrency permit acquired by `Runner::create_stream`.
    /// Released when this stream drops, freeing a slot for the next
    /// queued request.
    _permit: OwnedSemaphorePermit,
}

impl RunnerStream {
    pub(super) fn new(
        rx: mpsc::UnboundedReceiver<RunnerUpdate>,
        runner: Arc<Runner>,
        id: String,
        permit: OwnedSemaphorePermit,
    ) -> Self {
        Self {
            rx,
            runner,
            id,
            completed: false,
            _permit: permit,
        }
    }
}

impl Stream for RunnerStream {
    type Item = RunnerUpdate;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(update)) => {
                if matches!(
                    update,
                    RunnerUpdate::End(_)
                        | RunnerUpdate::Fatal(_)
                        | RunnerUpdate::RunnerExited
                ) {
                    self.completed = true;
                }
                Poll::Ready(Some(update))
            }
            Poll::Ready(None) => {
                // Channel closed by the runner-side reader after EOF
                // or after forwarding the terminal update; nothing
                // more will arrive.
                self.completed = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for RunnerStream {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        let runner = self.runner.clone();
        let id = std::mem::take(&mut self.id);
        // Drop is sync; spawn a fire-and-forget task to unregister.
        // The runner subprocess keeps producing output for this id
        // until natural completion; we just don't have anyone to
        // route it to anymore. Skip if we're not inside a tokio
        // runtime (e.g. the stream was dropped after the runtime
        // shut down).
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(async move {
                runner.unregister(&id).await;
            });
        }
    }
}
