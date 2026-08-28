//! The Claude Code subprocess: spawning it, and the queue verbs
//! against it.
//!
//! One run is one subprocess, and one global lock —
//! [`session::SESSION`] — holds everything a writer needs: stdin,
//! the queued uuids, and the RECEIVER for cancel replies, together.
//! The reader task permanently holds the matching sender, so the
//! lock's hold IS the protocol's atomicity. An enqueue locks,
//! writes, answers well. A dequeue locks and KEEPS the lock across
//! its cancel writes and its reply reads: nothing can enqueue while
//! it waits, so merely receiving the replies means the withdrawal
//! resolved.
//!
//! Deadlock audit: the reader never touches the session lock while
//! reading — only once, at end of stream, AFTER dropping the reply
//! sender — so a dequeue mid-wait always wakes (on `None` if the
//! run ends under it), and stdout always drains while an enqueue
//! blocks on a full stdin pipe.

mod delivered;
mod dequeue;
mod enqueue;
mod session;
mod spawn;
mod stdin;

pub use dequeue::dequeue;
pub use enqueue::enqueue;
// `spawn` itself has no caller until the root handler lands; the
// allow leaves with it.
#[allow(unused_imports)]
pub use spawn::spawn;
