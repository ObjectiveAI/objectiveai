//! Where the continuation lands, and where the run collects it.
//!
//! The keyless twin of a resource store: a run resumes from ONE
//! continuation, so there is one slot and nothing to name. The
//! server delivers it on the `/continuation` routes — chunks, then
//! the completion or the error — and the run's handler collects the
//! settled sequence with [`Store::fetch`], having first sent the ask
//! itself, on the socket it owns.
//!
//! Unlike a resource, a continuation is KEPT AS CHUNKS: the protocol
//! preserves the pieces an earlier run closed with, boundaries and
//! order intact, and a container may have put meaning in those
//! boundaries. So the slot holds a sequence, one entry per POST,
//! and hands the sequence back — it never joins.

use std::error;
use std::fmt;

use tokio::sync::{Mutex, Notify};

/// The one slot, alive as long as the container: deliveries arrive
/// whenever the server sends them, and the run reads whenever it is
/// ready — neither side waits on the other's schedule, so the
/// meeting place is a static.
pub static STORE: Store = Store {
    slot: Mutex::const_new(Entry::Assembling(Vec::new())),
    settled: Notify::const_new(),
};

/// The continuation being delivered: chunks kept as they arrive,
/// one entry each, settled by the completion or the error, collected
/// once by the run.
///
/// Whether a delivery was ever ASKED for is not judged here — the
/// server posts only in answer to the socket's ask, and an unasked
/// delivery is inert. What is judged is order: a settled slot takes
/// nothing more, because anything after a settlement is somebody's
/// bug.
pub struct Store {
    /// The slot. A tokio mutex, held for a push and never across an
    /// await.
    slot: Mutex<Entry>,
    /// Woken on the settlement, so a run waiting in
    /// [`fetch`](Self::fetch) re-checks.
    settled: Notify,
}

/// The delivery, in whichever state it has reached.
enum Entry {
    /// Chunks are landing; more may follow.
    Assembling(Vec<Vec<u8>>),
    /// The completion came: the chunks are the whole continuation —
    /// or none at all, which is the fresh start.
    Complete(Vec<Vec<u8>>),
    /// The error came: the bytes will never be whole, and this is
    /// the server's word on why. Whatever chunks preceded it went
    /// with it.
    Failed(serde_json::Value),
    /// The run collected it. A continuation is collected once.
    Taken,
}

impl Store {
    /// Keep one chunk, as its own entry. Answers whether it was taken
    /// — `false` means the delivery was already settled.
    pub async fn chunk(&self, body: &[u8]) -> bool {
        let mut slot = self.slot.lock().await;
        match &mut *slot {
            Entry::Assembling(chunks) => {
                chunks.push(body.to_vec());
                true
            }
            Entry::Complete(_) | Entry::Failed(_) | Entry::Taken => false,
        }
    }

    /// Settle the delivery whole: every chunk is in. A lone
    /// completion settles the fresh start. Answers whether it was
    /// taken — `false` means the delivery was already settled.
    pub async fn complete(&self) -> bool {
        self.settle(Entry::Complete).await
    }

    /// Settle the delivery failed: the bytes will never be whole,
    /// and the error is the server's word on why. Answers whether it
    /// was taken — `false` means the delivery was already settled.
    pub async fn error(&self, error: serde_json::Value) -> bool {
        self.settle(|_| Entry::Failed(error)).await
    }

    /// The one settlement, twice worn: swap the assembling slot for
    /// its ending and wake the waiter — after the lock drops.
    async fn settle(
        &self,
        ending: impl FnOnce(Vec<Vec<u8>>) -> Entry,
    ) -> bool {
        let taken = {
            let mut slot = self.slot.lock().await;
            match &mut *slot {
                Entry::Assembling(chunks) => {
                    *slot = ending(std::mem::take(chunks));
                    true
                }
                Entry::Complete(_) | Entry::Failed(_) | Entry::Taken => false,
            }
        };
        if taken {
            self.settled.notify_waiters();
        }
        taken
    }

    /// The continuation, settled — its chunks in delivery order,
    /// waiting for the completion or the error if neither has come
    /// yet, however long that takes (nothing in this protocol times
    /// anything out). `None` is the fresh start: the completion came
    /// with no chunks before it. Collected once; a second call is a
    /// bug, and says so.
    pub async fn fetch(&self) -> Result<Option<Vec<Vec<u8>>>, FetchError> {
        loop {
            // Armed before the check, so a settlement landing between
            // the check and the wait is a wakeup, not a lost one.
            let settled = self.settled.notified();
            {
                let mut slot = self.slot.lock().await;
                match &*slot {
                    Entry::Assembling(_) => {}
                    Entry::Complete(_) | Entry::Failed(_) => {
                        return match std::mem::replace(&mut *slot, Entry::Taken) {
                            Entry::Complete(chunks) => {
                                Ok((!chunks.is_empty()).then_some(chunks))
                            }
                            Entry::Failed(error) => Err(FetchError::Failed(error)),
                            Entry::Assembling(_) | Entry::Taken => {
                                unreachable!("checked settled above")
                            }
                        };
                    }
                    Entry::Taken => return Err(FetchError::Taken),
                }
            }
            settled.await;
        }
    }
}

/// A fetch that cannot answer.
#[derive(Debug)]
pub enum FetchError {
    /// The server failed the delivery — the bytes can never come
    /// (the client vanished mid-fetch, refused, …) — and this is its
    /// full error, verbatim off the error route.
    Failed(serde_json::Value),
    /// The continuation was already collected. One run, one fetch.
    Taken,
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Failed(error) => {
                write!(f, "the server failed the continuation: {error}")
            }
            FetchError::Taken => {
                f.write_str("the continuation was already collected")
            }
        }
    }
}

impl error::Error for FetchError {}
