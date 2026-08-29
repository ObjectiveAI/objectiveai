//! The writer lock.

use tokio::process;
use tokio::sync::Mutex;

/// The subprocess's stdin, or `None` before the run starts and after
/// it ends. Set by [`spawn`](super::spawn()), taken out by whoever
/// finds the process dead first — a failed write, a dequeue waking
/// on a closed reply channel, or the reader at end of stream.
///
/// Holding this lock is the sole right to write stdin AND to insert
/// into [`pending::PENDING`](super::pending::PENDING) — the pairing
/// is the invariant: a message's write and its fate's registration
/// happen under one hold, so a dequeue that keeps this lock sees the
/// pending map complete for everything written before it.
///
/// The lock is FIFO-fair, and that queue is the module's whole
/// ordering — the withdrawal boundary included: a dequeue's place in
/// the queue is the moment its cancel "came in". Every enqueue ahead
/// of it registers first and is withdrawn; every enqueue behind it
/// registers after the snapshot and is out of the withdrawal's
/// reach, never cancelled.
pub static WRITER: Mutex<Option<process::ChildStdin>> =
    Mutex::const_new(None);
