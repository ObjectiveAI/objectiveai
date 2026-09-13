//! The writes a scope has started and not yet been asked for.

use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use std::sync::Mutex;

use bytes::Bytes;
use futures_util::Stream;

use crate::shared::error::Error;

/// A write's content, as the caller supplied it: pieces in order,
/// or the failure that ends them.
pub type Content = Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + 'static>>;

/// The content of every write this end has opened a `write_path`
/// channel for and the provider has not yet asked for, by write id.
///
/// A write is two exchanges: the caller names the path on a channel
/// of its own, and the provider opens a channel back asking for the
/// content by the id the caller chose. This is where the content
/// waits between the two. Ids are minted here, unique among the
/// writes pending on the scope, and free again once the content has
/// been taken — a reused id after a write has finished is fine, since
/// nothing remembers.
pub struct Writes {
    pending: Mutex<Pending>,
}

/// What the lock protects.
struct Pending {
    next: u32,
    content: HashMap<u32, Content>,
}

impl Writes {
    pub(crate) fn new() -> Self {
        Writes {
            pending: Mutex::new(Pending {
                next: 0,
                content: HashMap::new(),
            }),
        }
    }

    /// Keep `content` until the provider asks for it, under a fresh
    /// id.
    pub(crate) fn register(&self, content: Content) -> u32 {
        let mut pending = self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut write_id = pending.next;
        while pending.content.contains_key(&write_id) {
            write_id = write_id.wrapping_add(1);
        }
        pending.next = write_id.wrapping_add(1);
        pending.content.insert(write_id, content);
        write_id
    }

    /// The content under `write_id`, if the provider's ask names a
    /// write this end started; taken, so a second ask finds nothing.
    pub(crate) fn take(&self, write_id: u32) -> Option<Content> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .content
            .remove(&write_id)
    }
}

impl fmt::Debug for Writes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pending = self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        f.debug_struct("Writes")
            .field("pending", &pending.content.len())
            .finish()
    }
}
