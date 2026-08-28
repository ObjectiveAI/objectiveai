//! The Claude Code subprocess: spawning it, and the queue verbs
//! against it.
//!
//! One run is one subprocess, behind two locks and a map:
//! [`writer::WRITER`] is stdin — holding it is the sole right to
//! write, and to register a fate; [`replies::REPLIES`] is the
//! RECEIVER for cancel replies, whose matching sender the reader
//! task permanently holds; and [`pending::PENDING`] is each enqueued
//! message's fate wire, inserted under the writer lock, resolved —
//! remove-and-send, lock-free — by whoever decides the fate.
//!
//! Fates are STRICT: an enqueue answers delivered, dequeued or
//! missed when that is truly known, not when its write lands. A
//! dequeue takes BOTH locks up front — joined, in parallel, the only
//! holder of the two at once — then holds each exactly as long as
//! its job: the writer through the cancel writes, so the pending map
//! it snapshots is the complete queue when the cancels land; the
//! replies through the reply reads, which stay answers to the
//! cancels written even as released enqueues flow — new messages
//! write no control responses. Ahead of it all, [`pending::GATE`]:
//! the FIFO boundary both verbs take first, so a message enqueued
//! after a withdrawal arrived is never the one withdrawn.
//!
//! Deadlock audit: the reader never touches a lock while reading —
//! only at end of stream, AFTER dropping the reply sender, and then
//! one lock at a time — so a dequeue mid-wait always wakes (on
//! `None` if the run ends under it), no lock-order cycle exists
//! against the dequeue's joined hold, and stdout always drains while
//! an enqueue blocks on a full stdin pipe.

mod dequeue;
mod enqueue;
mod pending;
mod replies;
mod spawn;
mod stdin;
mod writer;

pub use dequeue::dequeue;
pub use enqueue::enqueue;
// `spawn` itself has no caller until the root handler lands; the
// allow leaves with it.
#[allow(unused_imports)]
pub use spawn::spawn;
