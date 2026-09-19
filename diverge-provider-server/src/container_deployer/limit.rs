//! One cap, and the count against it.

use std::sync::atomic::{AtomicU64, Ordering};

/// A ceiling in BYTES and how much of it the running containers
/// hold: `container_overlay_disk` against the sum of their `disk`,
/// `memory` against the sum of their `memory`. Taken by one
/// compare-and-swap and never past the cap, given back when a
/// container ends; no lock, since every deploy takes from it beside
/// every other.
#[derive(Debug)]
pub struct Limit {
    /// The most the count may reach.
    cap: u64,
    /// The count, as of now.
    used: AtomicU64,
}

impl Limit {
    /// A limit of `cap`, with nothing taken.
    pub fn new(cap: u64) -> Self {
        Limit {
            cap,
            used: AtomicU64::new(0),
        }
    }

    /// Take `bytes`: `true` is the bytes taken, held until
    /// [`give`](Self::give)n back; `false` is a cap that would be
    /// passed, and nothing changed.
    pub fn take(&self, bytes: u64) -> bool {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (self.cap.saturating_sub(used) >= bytes).then(|| used + bytes)
            })
            .is_ok()
    }

    /// Give `bytes` back. Never takes the count below zero.
    pub fn give(&self, bytes: u64) {
        let _ = self
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| Some(used.saturating_sub(bytes)));
    }
}
