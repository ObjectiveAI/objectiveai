//! The MCP calls in flight on an agent: its own, and its dependency
//! tools'.

use std::sync::Arc;
use std::time::Instant;

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

    /// One file use begins — a watch, a download, an upload, a
    /// transfer on the container — counted in until the [`Use`] is
    /// dropped, which first touches the run through `touched`, so that
    /// its idle clock counts from the use's end.
    pub fn enter_use(self: &Arc<Self>, touched: watch::Sender<Instant>) -> Use {
        Use { touched, _guard: self.enter() }
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

/// One file use in progress. Dropping it touches the run, and only
/// then counts the use out: the idle clock reads the touch when it
/// sees the count change, so its countdown begins at the use's end.
#[derive(Debug)]
pub struct Use {
    touched: watch::Sender<Instant>,
    _guard: Guard,
}

impl Drop for Use {
    fn drop(&mut self) {
        self.touched.send_replace(Instant::now());
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.inflight.count.send_modify(|count| *count = count.saturating_sub(1));
    }
}
