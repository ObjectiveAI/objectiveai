//! The MCP calls in flight on an agent: its own, and its dependency
//! tools'.

use std::sync::Arc;

use tokio::sync::watch;

/// How many MCP exchanges are being answered now for one agent — a
/// list, a call, a read the agent's container asked for, or one of
/// its dependency tools did — kept as a watch so the idle clock sees
/// every change. One per agent run, shared with every dependency
/// deployed for it.
///
/// Why it is one of the two things that make an agent active: a loop
/// that has said its last chunk may still be waiting on a long tool
/// call, and a tool answering is a tool in use; the agent is not idle
/// until nothing is in flight.
#[derive(Debug)]
pub struct Inflight {
    count: watch::Sender<usize>,
}

impl Default for Inflight {
    fn default() -> Self {
        Inflight::new()
    }
}

impl Inflight {
    /// Nothing in flight.
    pub fn new() -> Self {
        Inflight {
            count: watch::channel(0).0,
        }
    }

    /// One exchange begins: counted in until the guard is dropped,
    /// which counts it out — at its answer, or at its future's end
    /// however that comes.
    pub fn enter(self: &Arc<Self>) -> Guard {
        self.count.send_modify(|count| *count += 1);
        Guard {
            inflight: Arc::clone(self),
        }
    }

    /// Whether anything is in flight now.
    pub fn is_busy(&self) -> bool {
        *self.count.borrow() > 0
    }

    /// Every change of the count from now on.
    pub fn subscribe(&self) -> watch::Receiver<usize> {
        self.count.subscribe()
    }
}

/// One exchange in flight; dropping it is the exchange over.
#[derive(Debug)]
pub struct Guard {
    inflight: Arc<Inflight>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.inflight.count.send_modify(|count| *count = count.saturating_sub(1));
    }
}
