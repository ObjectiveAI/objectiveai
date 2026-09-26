//! One cap, and the count against it: what the deployer keeps the
//! running containers under, and the volumes keep the ephemeral
//! serves under beside them.

use std::sync::atomic::{AtomicU64, Ordering};

/// A ceiling in BYTES and how much of it is held: `memory` against
/// the running containers' `memory`, and `container_overlay_disk`
/// against the running containers' `disk` and every ephemeral
/// serve's `overlay_disk` together, since a serve's scratch lives
/// beside the containers' overlays. Taken by one compare-and-swap
/// and never past the cap, given back when a container or a serve
/// ends; no lock, since every taker takes from it beside every
/// other.
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
