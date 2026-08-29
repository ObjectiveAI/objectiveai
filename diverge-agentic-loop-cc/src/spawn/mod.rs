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
//! holder of the two at once — and the writer's own FIFO queue is
//! the withdrawal boundary: a message enqueued after a withdrawal
//! joined that queue is never the one withdrawn. Each lock is held
//! exactly as long as its job: the writer through the cancel writes,
//! so the pending map it snapshots is the complete queue when the
//! cancels land; the replies through the reply reads, which stay
//! answers to the cancels written even as released enqueues flow —
//! new messages write no control responses.
//!
//! Error-typed records — a rejected rate limit, a failed auth
//! status, an error result, a line that failed the parse — always
//! travel as the stream's [`error::Error`], verdict unattached:
//! FATALITY IS FINALITY, the consumer's to decide by what follows.
//! An error before the run's first chunk is the request's own
//! failure (HTTP); one that anything at all follows was survivable
//! news (a non-fatal notification); the one the stream ends ON is
//! the run's death (the fatal final chunk).
//!
//! Deadlock audit: the reader never touches a lock while reading —
//! only at end of stream, AFTER dropping the reply sender, and then
//! one lock at a time — so a dequeue mid-wait always wakes (on
//! `None` if the run ends under it), no lock-order cycle exists
//! against the dequeue's joined hold, and stdout always drains while
//! an enqueue blocks on a full stdin pipe.

mod dequeue;
mod enqueue;
mod error;
mod install;
mod pending;
mod replies;
mod session_id;
mod spawn;
mod stdin;
mod writer;

pub use dequeue::dequeue;
pub use enqueue::enqueue;
pub use install::installed;
pub use session_id::session_id;
pub use spawn::spawn;
