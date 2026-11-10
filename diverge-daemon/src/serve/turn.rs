//! A request's turn in the order its connection sent it.

use tokio::sync::oneshot;

/// One request's link in its connection's chain: what makes the
/// messages one connection sends enter their agents' outboxes in the
/// order it sent them, whatever order their handlers' lookups finish
/// in. The rules:
///
/// 1. **One per request, in read order.** The connection's read loop
///    mints one for every request it reads, as it reads it and before
///    the request's task is spawned — [`Turns::next`] — and hands it
///    to [`dispatch`](super::dispatch). It asks nothing of the
///    request: a turn is minted for a request that is no message, for
///    one relayed on, and for one that cannot be read.
/// 2. **A turn comes once every turn before it is let go.** `before`
///    hears the link before it let go; the first of a connection has
///    none, and has come already. [`come`](Self::come) waits for it.
/// 3. **Letting go is the drop** — or [`pass`](Self::pass), which is
///    the same in place. Every request but a message to an agent this
///    daemon serves lets its turn go at once, in
///    [`dispatch`](super::dispatch). The message's handler makes its
///    lookups first, then waits for its turn to come, admits the
///    message, and lets go; one that returns early — forbidden, no
///    such agent — or panics lets go by the drop.
/// 4. **A turn let go before it came passes its place on.** The link
///    after it is let go only once every link before it is: the wait
///    is handed to a task of its own, since a drop cannot wait. So a
///    request that never waits — one that is no message, or a message
///    refused — never lets a later message past an earlier one still
///    making its lookups.
/// 5. **Nothing stalls the chain.** A link before that is gone
///    without a word — its sender dropped — is a link let go.
#[derive(Debug)]
pub struct Turn {
    /// The link before, until it is let go: `None` for the first of a
    /// connection, and once the turn has come or been passed on.
    before: Option<oneshot::Receiver<()>>,
    /// What the link after hears: dropped, never sent on. `None` once
    /// passed on.
    after: Option<oneshot::Sender<()>>,
}

/// A connection's chain of [`Turn`]s, as its read loop mints them.
#[derive(Debug, Default)]
pub struct Turns {
    /// What the next turn waits on: the last one minted's `after`.
    last: Option<oneshot::Receiver<()>>,
}

impl Turns {
    /// A chain with no turn minted.
    pub fn new() -> Self {
        Turns::default()
    }

    /// The next request's turn, behind every one minted before it:
    /// rule 1.
    pub fn next(&mut self) -> Turn {
        let (after, next) = oneshot::channel();
        Turn { before: self.last.replace(next), after: Some(after) }
    }
}

impl Turn {
    /// Wait for the turn to come: rule 2. Dropped midway, the turn is
    /// still waiting, and passes its place on as rule 4 states.
    pub async fn come(&mut self) {
        if let Some(before) = self.before.as_mut() {
            // Rule 5: a sender dropped is the word.
            let _ = before.await;
            self.before = None;
        }
    }

    /// Let the turn go, in place: rules 3 and 4. A turn that has come
    /// lets the next go now; one still waiting hands its wait on, and
    /// the next is let go when it ends. Nothing is done twice.
    pub fn pass(&mut self) {
        let after = self.after.take();
        let Some(mut before) = self.before.take() else {
            return;
        };
        if !matches!(before.try_recv(), Err(oneshot::error::TryRecvError::Empty)) {
            return;
        }
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _ = before.await;
                drop(after);
            });
        }
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        self.pass();
    }
}
