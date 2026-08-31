//! Where delivered resources land, and where the run collects them.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use tokio::sync::Notify;

/// The one store, alive as long as the container: deliveries arrive
/// on `/resource` whenever the server sends them, and the run reads
/// them whenever it is ready — neither side waits on the other's
/// schedule, so the meeting place is a static. Lazy because a
/// `HashMap` cannot be built in a const.
pub static STORE: LazyLock<Store> = LazyLock::new(Store::new);

/// Resources by identity: chunks appended as they arrive, sealed by
/// the completion, collected once by the run.
///
/// Whether a delivery was ever ASKED for is not judged here — the
/// server posts only in answer to the stream's asks, and an unasked
/// delivery is inert bytes in a map. What is judged is order: a
/// sealed identity takes nothing more, because bytes after "whole"
/// are somebody's bug.
pub struct Store {
    /// A std mutex, not tokio's: every hold is a few appends long
    /// and nothing awaits inside one.
    entries: Mutex<HashMap<String, Entry>>,
    /// Woken on every completion, so a run waiting in
    /// [`take`](Self::take) re-checks.
    sealed: Notify,
}

/// One resource being assembled.
struct Entry {
    /// What has arrived, in arrival order.
    bytes: Vec<u8>,
    /// Whether the completion came — the bytes are the whole
    /// resource.
    complete: bool,
}

impl Store {
    fn new() -> Self {
        Store {
            entries: Mutex::new(HashMap::new()),
            sealed: Notify::new(),
        }
    }

    /// Append one chunk. Answers whether it was taken — `false`
    /// means the identity was already sealed.
    pub fn chunk(&self, identity: &str, body: &[u8]) -> bool {
        let mut entries = self.entries.lock().expect("the store is poisoned");
        let entry =
            entries.entry(identity.to_string()).or_insert_with(|| Entry {
                bytes: Vec::new(),
                complete: false,
            });
        if entry.complete {
            return false;
        }
        entry.bytes.extend_from_slice(body);
        true
    }

    /// Seal one identity: every chunk is in. Answers whether it was
    /// taken — `false` means the identity was already sealed. A lone
    /// completion seals the empty resource.
    pub fn complete(&self, identity: &str) -> bool {
        let mut entries = self.entries.lock().expect("the store is poisoned");
        let entry =
            entries.entry(identity.to_string()).or_insert_with(|| Entry {
                bytes: Vec::new(),
                complete: false,
            });
        if entry.complete {
            return false;
        }
        entry.complete = true;
        drop(entries);
        self.sealed.notify_waiters();
        true
    }

    /// The resource behind one identity, whole — waiting for the
    /// completion if it has not come yet, however long that takes
    /// (nothing in this protocol times anything out). Removes the
    /// entry: a resource is collected once, by the one run.
    // The run that calls this is not implemented yet; the collector
    // is the store's other half, not dead weight.
    #[allow(dead_code)]
    pub async fn take(&self, identity: &str) -> Vec<u8> {
        loop {
            // Armed before the check, so a completion landing between
            // the check and the wait is a wakeup, not a lost one.
            let sealed = self.sealed.notified();
            {
                let mut entries =
                    self.entries.lock().expect("the store is poisoned");
                if entries.get(identity).is_some_and(|entry| entry.complete)
                {
                    return entries
                        .remove(identity)
                        .expect("checked present above")
                        .bytes;
                }
            }
            sealed.await;
        }
    }
}
