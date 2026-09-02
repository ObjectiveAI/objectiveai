//! Where the continuation lands, and where the run collects it.
//!
//! The keyless twin of a resource store: a run resumes from ONE
//! continuation, so there is one slot and nothing to name. The
//! server delivers it on the `/continuation` routes — chunks, then
//! the completion or the error — and the run's handler waits out the
//! settlement with [`Store::fetch`], having first sent the ask
//! itself, on the socket it owns.
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

use crate::continuation;
use crate::continuation::{CheckError, Ingest, IngestError};

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
    /// [`fetch`](Self::fetch) re-checks.
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

    /// Whether a continuation is on disk — waiting for the completion
    /// or the error if neither has come yet, however long that takes
    /// (nothing in this protocol times anything out). `false` is the
    /// fresh start: the completion came with no chunks before it. A
    /// delivered database is proved to open ([`continuation::check`])
    /// before `true` is answered. Collected once; a second call is a
    /// bug, and says so.
    pub async fn fetch(&self) -> Result<bool, FetchError> {
        loop {
            // Armed before the check, so a settlement landing between
            // the check and the wait is a wakeup, not a lost one.
            let settled = self.settled.notified();
            let taken = {
                let mut slot = self.slot.lock().await;
                match &*slot {
                    Entry::Assembling(_) => None,
                    Entry::Taken => return Err(FetchError::Taken),
                    Entry::Complete(_) | Entry::Failed(_) => {
                        Some(std::mem::replace(&mut *slot, Entry::Taken))
                    }
                }
            };
            match taken {
                None => settled.await,
                Some(Entry::Complete(resumed)) => {
                    if resumed {
                        continuation::check().await?;
                    }
                    return Ok(resumed);
                }
                Some(Entry::Failed(Failure::Server(error))) => {
                    return Err(FetchError::Failed(error));
                }
                Some(Entry::Failed(Failure::Ingest(error))) => {
                    return Err(FetchError::Ingest(error));
                }
                Some(Entry::Assembling(_) | Entry::Taken) => {
                    unreachable!("only settled entries are taken")
                }
            }
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
    /// The bytes came but could not be landed.
    Ingest(IngestError),
    /// The bytes landed but the database did not check out.
    Check(CheckError),
    /// The continuation was already collected. One run, one fetch.
    Taken,
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Failed(error) => {
                write!(f, "the server failed the continuation: {error}")
            }
            FetchError::Ingest(error) => {
                write!(f, "the continuation could not be landed: {error}")
            }
            FetchError::Check(error) => write!(f, "{error}"),
            FetchError::Taken => {
                f.write_str("the continuation was already collected")
            }
        }
    }
}

impl error::Error for FetchError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FetchError::Ingest(error) => Some(error),
            FetchError::Check(error) => Some(error),
            FetchError::Failed(_) | FetchError::Taken => None,
        }
    }
}

impl From<CheckError> for FetchError {
    fn from(error: CheckError) -> Self {
        FetchError::Check(error)
    }
}
