//! Where the continuation lands, and where the run collects it.
//!
//! The keyless twin of the [resource store](super::resource): a run
//! resumes from ONE continuation, so there is one slot and nothing
//! to name. The server delivers it on the `/continuation` routes —
//! chunks, then the completion or the error — and the
//! [`Fetcher`](crate::fetcher::Fetcher) collects it with
//! [`Store::take`], having sent the ask.
//!
//! # The slot holds none of it
//!
//! A continuation can be large — a lineage's whole database and its
//! memories — and is never held in memory. Each chunk goes to disk
//! the moment it lands, appended to the file its tag names by the
//! [`Ingest`] the slot keeps open; what the slot remembers is only
//! the ingest itself and, once settled, whether anything landed. The
//! run then finds its state on the filesystem, where Hermes reads it.

use std::error;
use std::fmt;

use tokio::sync::{Mutex, Notify};

use crate::fetcher::ContinuationError;
use crate::filesystem::continuation;
use crate::filesystem::continuation::{Ingest, IngestError};

/// The one slot, alive as long as the container: deliveries arrive
/// whenever the server sends them, and the run reads whenever it is
/// ready — neither side waits on the other's schedule, so the
/// meeting place is a static.
pub static STORE: Store = Store {
    slot: Mutex::const_new(Entry::Assembling(None)),
    settled: Notify::const_new(),
};

/// The continuation being delivered: written to disk as it arrives,
/// settled by the completion or the error, collected once by the
/// run.
///
/// Whether a delivery was ever ASKED for is not judged here — the
/// server posts only in answer to the socket's ask, and an unasked
/// delivery is inert. What is judged is order: a settled slot takes
/// nothing more, because anything after a settlement is somebody's
/// bug.
pub struct Store {
    /// The slot. A tokio mutex, because it is held across the file
    /// writes each chunk costs.
    slot: Mutex<Entry>,
    /// Woken on the settlement, so a run waiting in
    /// [`take`](Self::take) re-checks.
    settled: Notify,
}

/// The delivery, in whichever state it has reached.
enum Entry {
    /// Chunks may land; more may follow. `None` until the first one
    /// starts the ingest — so a fresh start touches no file.
    Assembling(Option<Ingest>),
    /// The completion came: `true`, the files are on disk; `false`,
    /// nothing landed — the fresh start.
    Complete(bool),
    /// The delivery failed, one way or the other.
    Failed(Failure),
    /// The run collected it. A continuation is collected once.
    Taken,
}

/// The two ways a delivery fails.
enum Failure {
    /// The server said the bytes can never come — this is its word
    /// on why, verbatim off the error route.
    Server(serde_json::Value),
    /// The bytes came but could not be landed: a chunk broke the
    /// format's rules, or a file would not take them.
    Ingest(IngestError),
}

impl Store {
    /// Land one chunk: onto disk, into the file its tag names.
    /// Answers whether it was taken — `false` means the delivery was
    /// already settled. A chunk that cannot be landed is taken too,
    /// and settles the slot as failed: what was wrong with it is
    /// this container's own format, and the run reports it.
    pub async fn chunk(&self, body: &[u8]) -> bool {
        let mut slot = self.slot.lock().await;
        let Entry::Assembling(ingest) = &mut *slot else {
            return false;
        };
        let landed = async {
            if ingest.is_none() {
                *ingest = Some(Ingest::start().await?);
            }
            ingest.as_mut().expect("started just above").push(body).await
        }
        .await;
        if let Err(error) = landed {
            *slot = Entry::Failed(Failure::Ingest(error));
            drop(slot);
            self.settled.notify_waiters();
        }
        true
    }

    /// Settle the delivery whole: every chunk is in. A lone
    /// completion settles the fresh start. Answers whether it was
    /// taken — `false` means the delivery was already settled.
    pub async fn complete(&self) -> bool {
        let mut slot = self.slot.lock().await;
        let Entry::Assembling(ingest) = &mut *slot else {
            return false;
        };
        *slot = match ingest.take() {
            None => Entry::Complete(false),
            Some(ingest) => match ingest.finish().await {
                Ok(()) => Entry::Complete(true),
                Err(error) => Entry::Failed(Failure::Ingest(error)),
            },
        };
        drop(slot);
        self.settled.notify_waiters();
        true
    }

    /// Settle the delivery failed: the bytes will never be whole,
    /// and the error is the server's word on why. Whatever landed
    /// already is abandoned where it lies — the run fails, and
    /// nothing reads it. Answers whether it was taken — `false`
    /// means the delivery was already settled.
    pub async fn error(&self, error: serde_json::Value) -> bool {
        let mut slot = self.slot.lock().await;
        if !matches!(*slot, Entry::Assembling(_)) {
            return false;
        }
        *slot = Entry::Failed(Failure::Server(error));
        drop(slot);
        self.settled.notify_waiters();
        true
    }

    /// The session to resume, once the continuation is on disk —
    /// waiting for the completion or the error if neither has come
    /// yet, however long that takes (nothing in this protocol times
    /// anything out). `None` is the fresh start: the completion came
    /// with no chunks before it. Otherwise the delivered database is
    /// proved to open and asked for its most recently active session
    /// ([`continuation::check`]), and that id is the answer.
    /// Collected once; a second call is a bug, and says so.
    pub async fn take(&self) -> Result<Option<String>, ContinuationError> {
        loop {
            // Armed before the check, so a settlement landing between
            // the check and the wait is a wakeup, not a lost one.
            let settled = self.settled.notified();
            let taken = {
                let mut slot = self.slot.lock().await;
                match &*slot {
                    Entry::Assembling(_) => None,
                    Entry::Taken => return Err(ContinuationError::Taken),
                    Entry::Complete(_) | Entry::Failed(_) => {
                        Some(std::mem::replace(&mut *slot, Entry::Taken))
                    }
                }
            };
            match taken {
                None => settled.await,
                Some(Entry::Complete(landed)) => {
                    return if landed {
                        Ok(Some(continuation::check().await?))
                    } else {
                        Ok(None)
                    };
                }
                Some(Entry::Failed(Failure::Server(error))) => {
                    return Err(ContinuationError::Failed(error));
                }
                Some(Entry::Failed(Failure::Ingest(error))) => {
                    return Err(ContinuationError::Ingest(error));
                }
                Some(Entry::Assembling(_) | Entry::Taken) => {
                    unreachable!("only settled entries are taken")
                }
            }
        }
    }
}
