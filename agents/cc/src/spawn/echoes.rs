//! The batches written and not yet echoed.

use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use tokio::sync::Mutex;

/// One batch written to the running session as one user message, and
/// not yet echoed back: the proxy's batch as enqueued — what the user
/// parts carry, each message's under its own key, when the replay
/// echo lands, since the echo carries Claude Code's blocks and not
/// MCP's — under the uuid its line carried.
pub struct Written {
    /// The correlation key the replay echo carries back.
    pub uuid: String,
    /// The batch, in enqueue order, each message under its key.
    pub messages: Vec<Message>,
}

/// Every batch written and not yet echoed, in the order written.
///
/// Pushed by an enqueue UNDER the writer lock and BEFORE its write —
/// so no echo can arrive for a batch not yet here — and taken out by
/// the reader, by uuid, when the echo lands. Its own lock, held for
/// a push, a removal or a clearing and never across a write, so the
/// reader taking it can never wait on an enqueue blocked on a full
/// stdin pipe. Cleared at the run's end: a batch the run never took
/// is said by nobody — it was delivered, the program's, when its
/// write landed.
pub static ECHOES: Mutex<Vec<Written>> = Mutex::const_new(Vec::new());
