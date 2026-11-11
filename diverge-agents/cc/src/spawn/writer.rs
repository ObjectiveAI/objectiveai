//! The writer lock.

use tokio::process;
use tokio::sync::Mutex;

/// The subprocess's stdin, or `None` before the run starts and after
/// it ends. Set by [`spawn`](super::spawn()), taken out by whoever
/// finds the process dead first — a failed write, or the reader at
/// end of stream.
///
/// Holding this lock is the sole right to write stdin, and `None`
/// here is the run closed: an enqueue checks and writes under one
/// hold, so nothing is ever written into a run that has ended, and
/// an enqueue that finds it `None` is refused as closed. The lock is
/// FIFO-fair, so enqueues write in the order they arrived.
pub static WRITER: Mutex<Option<process::ChildStdin>> =
    Mutex::const_new(None);
