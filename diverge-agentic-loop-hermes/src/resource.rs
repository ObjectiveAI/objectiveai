//! Where delivered resources land, and where the run collects them.

use std::sync::LazyLock;

use dashmap::DashMap;
use tokio::sync::Notify;

/// The one store, alive as long as the container: deliveries arrive
/// on the `/resource/{identity}` routes whenever the server sends
/// them, and the run reads them whenever it is ready — neither side
/// waits on the other's schedule, so the meeting place is a static.
/// Lazy because a `DashMap` cannot be built in a const.
pub static STORE: LazyLock<Store> = LazyLock::new(Store::new);

/// Resources by identity: chunks appended as they arrive, settled
/// by the completion or the error, collected once by the run.
///
/// A [`DashMap`], not a map behind one mutex: the map shards by
/// key, so concurrent deliveries of DIFFERENT identities append in
/// parallel instead of queuing on a single lock — same-identity
/// POSTs are sequential by the server's own one-at-a-time rule
/// anyway. No shard guard is ever held across an await; the one
/// wait, [`take`](Self::take)'s, sits outside the map.
///
/// Whether a delivery was ever ASKED for is not judged here — the
/// server posts only in answer to the stream's asks, and an unasked
/// delivery is inert in a map. What is judged is order: a settled
/// identity takes nothing more, because anything after a settlement
/// is somebody's bug.
pub struct Store {
    /// The resources, sharded by identity.
    entries: DashMap<String, Entry>,
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
            entries: DashMap::new(),
            settled: Notify::new(),
        }
    }

    /// Append one chunk. Answers whether it was taken — `false`
    /// means the identity was already settled.
    pub fn chunk(&self, identity: &str, body: &[u8]) -> bool {
        let mut entry = self
            .entries
            .entry(identity.to_string())
            .or_insert_with(|| Entry::Assembling(Vec::new()));
        match entry.value_mut() {
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
    /// absent) entry for its ending and wake the waiters — after
    /// the shard guard drops, so a woken waiter reaching for the
    /// map never meets it.
    fn settle(
        &self,
        identity: &str,
        ending: impl FnOnce(Vec<u8>) -> Entry,
    ) -> bool {
        let taken = {
            let mut entry = self
                .entries
                .entry(identity.to_string())
                .or_insert_with(|| Entry::Assembling(Vec::new()));
            let value = entry.value_mut();
            match value {
                Entry::Assembling(bytes) => {
                    *value = ending(std::mem::take(bytes));
                    true
                }
                Entry::Complete(_) | Entry::Failed(_) => false,
            }
        };
        if taken {
            self.settled.notify_waiters();
        }
        taken
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
            // Removal only if settled, atomically — an assembling
            // entry stays where the next chunk expects it.
            if let Some((_, entry)) =
                self.entries.remove_if(identity, |_, entry| {
                    matches!(entry, Entry::Complete(_) | Entry::Failed(_))
                })
            {
                return match entry {
                    Entry::Complete(bytes) => Ok(bytes),
                    Entry::Failed(error) => Err(error),
                    Entry::Assembling(_) => {
                        unreachable!("remove_if took only settled entries")
                    }
                };
            }
            settled.await;
        }
    }
}
