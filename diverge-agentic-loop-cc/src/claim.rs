//! The run lock: one run at a time, held for exactly the stream's
//! life.

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether a run is in progress.
///
/// One at a time, not one per container: the queue, the history and
/// the MCP session are one conversation's, and a second run beside
/// the first would share all of them — but a run AFTER the first
/// resumes the same conversation, which is what the history is for.
static CLAIMED: AtomicBool = AtomicBool::new(false);

/// The run lock, held.
///
/// Taken by the first `/run` to arrive while none is in progress — an
/// atomic swap, so two arrivals a nanosecond apart resolve to exactly
/// one winner — and released the instant it drops: on every refusal
/// before the stream exists, and otherwise with the stream itself,
/// which captures it, so a run that finished and a run whose caller
/// left both free the container at once. Nothing awaits in the
/// release, so "at once" is literal.
#[must_use = "dropping the claim releases the run lock"]
pub struct Claim(());

impl Claim {
    /// Take the lock, or learn that a run is in progress.
    pub fn take() -> Option<Self> {
        if CLAIMED.swap(true, Ordering::SeqCst) {
            None
        } else {
            Some(Claim(()))
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        CLAIMED.store(false, Ordering::SeqCst);
    }
}
