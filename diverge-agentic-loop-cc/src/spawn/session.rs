//! The one lock: the writer, and the reply receiver beside it.

use tokio::process;
use tokio::sync::{Mutex, mpsc};

use crate::response;

/// The running session, or `None` before the run starts and after it
/// ends. Set by [`spawn`](super::spawn()), taken out by whoever finds
/// the process dead first — a failed write, a dequeue waking on a
/// closed reply channel, or the reader at end of stream.
///
/// The lock's hold IS the protocol's atomicity: writing to the
/// subprocess, reading the reply channel, and adding to the pending
/// map are rights only the lock holder has, and everything the queue
/// verbs promise follows from that exclusivity.
pub static SESSION: Mutex<Option<Session>> = Mutex::const_new(None);

/// What the one lock protects.
pub struct Session {
    /// The subprocess's stdin: the only way in.
    pub stdin: process::ChildStdin,
    /// Where the reader's forwarded cancel replies arrive. The
    /// receiver lives INSIDE the lock so that reading it is a right
    /// only the lock holder has — which is what lets a dequeue treat
    /// the replies it reads as answers to the cancels it wrote.
    pub replies:
        mpsc::UnboundedReceiver<response::control::ControlResponse>,
}
