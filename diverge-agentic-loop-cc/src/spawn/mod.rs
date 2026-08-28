//! The Claude Code subprocess: spawning it, and the queue verbs
//! against it.
//!
//! One run is one subprocess, and one global lock —
//! [`session::SESSION`] — holds everything a writer needs: stdin and
//! the RECEIVER for cancel replies, together. The reader task
//! permanently holds the matching sender, so the lock's hold IS the
//! protocol's atomicity. Beside the lock, [`pending::PENDING`]: each
//! enqueued message's fate wire, inserted under the lock, resolved —
//! remove-and-send, lock-free — by whoever decides the fate. Fates
//! are STRICT: an enqueue answers delivered, dequeued or missed when
//! that is truly known, not when its write lands. A dequeue locks
//! and KEEPS the lock across its cancel writes and its reply reads:
//! nothing can enqueue while it waits, so the pending map is the
//! complete queue and the replies read are answers to the cancels
//! written.
//!
//! Deadlock audit: the reader never touches the session lock while
//! reading — only once, at end of stream, AFTER dropping the reply
//! sender — so a dequeue mid-wait always wakes (on `None` if the
//! run ends under it), and stdout always drains while an enqueue
//! blocks on a full stdin pipe.

mod dequeue;
mod enqueue;
mod pending;
mod session;
mod spawn;
mod stdin;

pub use dequeue::dequeue;
pub use enqueue::enqueue;
// `spawn` itself has no caller until the root handler lands; the
// allow leaves with it.
#[allow(unused_imports)]
pub use spawn::spawn;
