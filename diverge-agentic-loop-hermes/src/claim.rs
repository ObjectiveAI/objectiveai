//! The run lock: one run at a time, and never a refusal for a run
//! that is merely still settling.

use std::pin::pin;
use std::sync::Mutex;

use tokio::sync::Notify;

/// Where the container is between runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// No run: the next request takes the lock at once.
    Idle,
    /// A run is streaming: the next request is refused.
    Running,
    /// A run's stream has ended and its settlement — the queue's
    /// locks cleared, its fates decided — is on a task that has not
    /// finished yet: the next request WAITS for it, and then takes
    /// the lock. Never a refusal: a settlement takes microseconds,
    /// and a request that happened to land inside them is not a
    /// parallel run.
    Settling,
}

/// The phase, and the bell rung when a settlement finishes.
static PHASE: Mutex<Phase> = Mutex::new(Phase::Idle);
static SETTLED: Notify = Notify::const_new();

/// The run lock, held.
///
/// One run at a time, not one per container: the queue, the session
/// and the filesystem are one conversation's, and a second run beside
/// the first would share all of them — but a run AFTER the first
/// resumes the same conversation, which is what the session is for.
///
/// Taken by [`take`](Self::take); the phase is `Running` while it is
/// held. The run's stream ending turns the phase to `Settling` by
/// [`settling`](Self::settling), before the settlement task starts;
/// dropping the claim — the settlement task's last act — turns it to
/// `Idle` and wakes every request waiting on it. A claim dropped on a
/// refusal before any stream existed goes straight from `Running` to
/// `Idle`, since there was nothing to settle.
#[must_use = "dropping the claim releases the run lock"]
pub struct Claim(());

/// A run is in progress: the request is refused, not queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

impl Claim {
    /// Take the lock: at once when the container is idle, after the
    /// settlement when one is in flight, or not at all — [`Busy`] —
    /// when a run is streaming.
    ///
    /// The bell is armed BEFORE the phase is read, so a settlement
    /// finishing between the read and the wait is not missed.
    pub async fn take() -> Result<Self, Busy> {
        loop {
            let notified = SETTLED.notified();
            let mut notified = pin!(notified);
            notified.as_mut().enable();
            {
                let mut phase = PHASE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                match *phase {
                    Phase::Idle => {
                        *phase = Phase::Running;
                        return Ok(Claim(()));
                    }
                    Phase::Running => return Err(Busy),
                    Phase::Settling => {}
                }
            }
            notified.await;
        }
    }

    /// The run's stream has ended and its settlement is about to run:
    /// requests from here on wait instead of being refused.
    pub fn settling(&self) {
        let mut phase = PHASE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *phase = Phase::Settling;
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        {
            let mut phase = PHASE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            *phase = Phase::Idle;
        }
        SETTLED.notify_waiters();
    }
}
