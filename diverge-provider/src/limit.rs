//! One cap, and the count against it: what the deployer keeps the
//! running containers under, and the volumes keep the ephemeral
//! serves under beside them.

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// What one permit of the cap is worth, in bytes: the cap and every
/// take are counted in these, which is what lets a take of many
/// gibibytes be one acquire — a semaphore counts in permits, and
/// takes at most `u32::MAX` of them at once — and costs nothing a
/// container or a scratch file could notice, since both are measured
/// in whole pages anyway.
const UNIT: u64 = 4096;

/// A ceiling in BYTES and how much of it is held: `memory` against
/// the running containers' `memory`, and `container_overlay_disk`
/// against the running containers' `disk` and every ephemeral
/// serve's `overlay_disk` together, since a serve's scratch lives
/// beside the containers' overlays. A take is a [`Held`], and the
/// bytes go back when it is dropped — by whoever held it, however it
/// ended — so nothing has to remember to give.
#[derive(Debug)]
pub struct Limit {
    /// The cap, as permits of [`UNIT`] bytes.
    permits: Arc<Semaphore>,
}

/// Bytes taken from a [`Limit`], given back on drop.
#[derive(Debug)]
pub struct Held {
    /// The permits, which the semaphore takes back when this is
    /// dropped.
    _permits: OwnedSemaphorePermit,
}

impl Limit {
    /// A limit of `cap` bytes, with nothing taken. A cap beyond what a
    /// semaphore counts is the most it counts.
    pub fn new(cap: u64) -> Self {
        let permits = usize::try_from(cap / UNIT).unwrap_or(usize::MAX).min(Semaphore::MAX_PERMITS);
        Limit {
            permits: Arc::new(Semaphore::new(permits)),
        }
    }

    /// Take `bytes`, rounded up to whole units: `Some` is the bytes
    /// held until the [`Held`] is dropped; `None` is a cap that would
    /// be passed, or a take too large to count at once, and nothing
    /// changed.
    pub fn take(&self, bytes: u64) -> Option<Held> {
        let permits = u32::try_from(bytes.div_ceil(UNIT)).ok()?;
        Arc::clone(&self.permits)
            .try_acquire_many_owned(permits)
            .ok()
            .map(|permits| Held { _permits: permits })
    }
}
