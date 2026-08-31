//! Where delivered resources land, and where the run collects them.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use tokio::sync::Notify;

/// The one store, alive as long as the container: deliveries arrive
/// on the `/resource/{identity}` routes whenever the server sends
/// them, and the run reads them whenever it is ready — neither side
/// waits on the other's schedule, so the meeting place is a static.
/// Lazy because a `HashMap` cannot be built in a const.
pub static STORE: LazyLock<Store> = LazyLock::new(Store::new);

/// Resources by identity: chunks appended as they arrive, settled
/// by the completion or the error, collected once by the run.
///
/// Whether a delivery was ever ASKED for is not judged here — the
/// server posts only in answer to the stream's asks, and an unasked
/// delivery is inert in a map. What is judged is order: a settled
/// identity takes nothing more, because anything after a settlement
/// is somebody's bug.
pub struct Store {
    /// A std mutex, not tokio's: every hold is a few appends long
    /// and nothing awaits inside one.
    entries: Mutex<HashMap<String, Entry>>,
    /// Woken on every settlement, so a run waiting in
    /// [`take`](Self::take) re-checks.
    settled: Notify,
}

/// One resource, in whichever state its delivery has reached.
enum Entry {
    /// Chunks are landing; more may follow.
    Assembling(Vec<u8>),
    /// The completion came: the bytes are the whole resource.
    Complete(Vec<u8>),
    /// The error came: the bytes will never be whole, and this is
    /// the server's word on why. Whatever chunks preceded it went
    /// with it — a partial fails the identity's size and hash
    /// anyway.
    Failed(serde_json::Value),
}

impl Store {
    fn new() -> Self {
        Store {
            entries: Mutex::new(HashMap::new()),
            settled: Notify::new(),
        }
    }

    /// Append one chunk. Answers whether it was taken — `false`
    /// means the identity was already settled.
    pub fn chunk(&self, identity: &str, body: &[u8]) -> bool {
        let mut entries = self.entries.lock().expect("the store is poisoned");
        let entry = entries
            .entry(identity.to_string())
            .or_insert_with(|| Entry::Assembling(Vec::new()));
        match entry {
            Entry::Assembling(bytes) => {
                bytes.extend_from_slice(body);
                true
            }
            Entry::Complete(_) | Entry::Failed(_) => false,
        }
    }

    /// Settle one identity whole: every chunk is in. Answers
    /// whether it was taken — `false` means the identity was
    /// already settled. A lone completion settles the empty
    /// resource.
    pub fn complete(&self, identity: &str) -> bool {
        self.settle(identity, Entry::Complete)
    }

    /// Settle one identity failed: the bytes will never be whole,
    /// and the error is the server's word on why. Answers whether
    /// it was taken — `false` means the identity was already
    /// settled.
    pub fn error(&self, identity: &str, error: serde_json::Value) -> bool {
        self.settle(identity, |_| Entry::Failed(error))
    }

    /// The one settlement, twice worn: swap an assembling (or
    /// absent) entry for its ending and wake the waiters. A settled
    /// entry refuses.
    fn settle(
        &self,
        identity: &str,
        ending: impl FnOnce(Vec<u8>) -> Entry,
    ) -> bool {
        let mut entries = self.entries.lock().expect("the store is poisoned");
        let entry = entries
            .entry(identity.to_string())
            .or_insert_with(|| Entry::Assembling(Vec::new()));
        match entry {
            Entry::Assembling(bytes) => {
                *entry = ending(std::mem::take(bytes));
                drop(entries);
                self.settled.notify_waiters();
                true
            }
            Entry::Complete(_) | Entry::Failed(_) => false,
        }
    }

    /// The resource behind one identity, settled — waiting for the
    /// completion or the error if neither has come yet, however
    /// long that takes (nothing in this protocol times anything
    /// out). The whole bytes, or the server's error. Removes the
    /// entry: a resource is collected once, by the one run.
    // The run that calls this is not implemented yet; the collector
    // is the store's other half, not dead weight.
    #[allow(dead_code)]
    pub async fn take(
        &self,
        identity: &str,
    ) -> Result<Vec<u8>, serde_json::Value> {
        loop {
            // Armed before the check, so a settlement landing
            // between the check and the wait is a wakeup, not a
            // lost one.
            let settled = self.settled.notified();
            {
                let mut entries =
                    self.entries.lock().expect("the store is poisoned");
                match entries.get(identity) {
                    Some(Entry::Complete(_) | Entry::Failed(_)) => {
                        return match entries
                            .remove(identity)
                            .expect("checked present above")
                        {
                            Entry::Complete(bytes) => Ok(bytes),
                            Entry::Failed(error) => Err(error),
                            Entry::Assembling(_) => {
                                unreachable!("checked settled above")
                            }
                        };
                    }
                    Some(Entry::Assembling(_)) | None => {}
                }
            }
            settled.await;
        }
    }
}
