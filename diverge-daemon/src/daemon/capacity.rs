//! One provider's latest capacity: swapped in by its poller, read by
//! anyone at any time, and nudged by whoever changes it.

use std::sync::Arc;

use arc_swap::ArcSwapOption;
use diverge_sdk::provider::endpoints::capacity::server::response::Frame;
use tokio::sync::Notify;

/// A connected provider's latest answer to the daemon's `capacity`
/// ask, and the word that it should be asked again.
///
/// # Read under no lock
///
/// The answer sits behind an atomic swap: a reader loads what is
/// there, at any time, never blocked by the writer; the poller
/// swaps a new answer in, and every load after the swap sees it.
/// There is one writer, the provider's poller, so there is nothing
/// to serialize but the swap itself.
///
/// # Nudged, not awaited
///
/// Whoever does something that changes the provider's capacity — a
/// run started or ended, a volume made, resized or dropped, a FUSE
/// serve opened or closed — nudges the cell, and the poller asks
/// again at once instead of waiting out its interval. A nudge keeps
/// one permit, so one that arrives while the poller is mid-ask fires
/// its next wait at once; nobody waits for the answer.
#[derive(Debug)]
pub struct Capacity {
    /// The latest answer; `None` until the first.
    latest: ArcSwapOption<Frame>,
    /// Ask again, now.
    nudge: Notify,
}

impl Capacity {
    /// An empty cell, nothing answered yet.
    pub fn new() -> Self {
        Capacity {
            latest: ArcSwapOption::empty(),
            nudge: Notify::new(),
        }
    }

    /// The latest answer, as of now.
    pub fn latest(&self) -> Option<Arc<Frame>> {
        self.latest.load_full()
    }

    /// The answer swapped in: whether it differs from what was there,
    /// for whoever tells the listings; the same answer again is
    /// nothing to say.
    pub fn swap(&self, frame: Frame) -> bool {
        let same = self.latest.load().as_deref() == Some(&frame);
        self.latest.store(Some(Arc::new(frame)));
        !same
    }

    /// Ask again, now.
    pub fn nudge(&self) {
        self.nudge.notify_one();
    }

    /// Wait for a nudge.
    pub async fn nudged(&self) {
        self.nudge.notified().await;
    }
}

impl Default for Capacity {
    fn default() -> Self {
        Capacity::new()
    }
}
