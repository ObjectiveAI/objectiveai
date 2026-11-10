//! The clock: every due alarm fired, for the container's life.
//!
//! One task, beside the server. Each round it:
//! 1. reaches the database, trying again every [`RETRY`] until the
//!    proxy answers;
//! 2. fires every alarm due by now, inside one transaction that holds
//!    their rows: each is sent to its agent, then a one-shot alarm is
//!    deleted and a recurring one moved to its next time — whether the
//!    message went or not, since an alarm that cannot be sent when due
//!    is dropped;
//! 3. sleeps until the soonest alarm left is due, or until a tool call
//!    sets or cancels one, whichever comes first; with no alarm left,
//!    until a call alone.
//!
//! An alarm that fell due while the tool was down fires once, on the
//! first round after it comes back. A round that fails partway tries
//! again after [`RETRY`]; its transaction is rolled back, so nothing
//! it settled is lost, though a message it sent may be sent again.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::alarm::State;
use crate::failure::Failure;
use crate::schedule;
use crate::store;

/// How long the scheduler waits before trying again, when the
/// database cannot be reached or a round fails.
pub const RETRY: Duration = Duration::from_secs(5);

/// Run the clock, forever.
pub async fn run(state: Arc<State>) {
    loop {
        let next = match state.store.pool(&state.client).await {
            Ok(pool) => round(&state, pool).await,
            Err(failure) => Err(failure),
        };
        match next {
            Ok(Some(due)) => {
                let wait = (due - Utc::now()).to_std().unwrap_or_default();
                tokio::select! {
                    () = tokio::time::sleep(wait) => {}
                    () = state.wake.notified() => {}
                }
            }
            Ok(None) => state.wake.notified().await,
            Err(_) => tokio::time::sleep(RETRY).await,
        }
    }
}

/// One round: every alarm due by now fired and settled; when the
/// soonest left is due.
async fn round(state: &State, pool: &PgPool) -> Result<Option<DateTime<Utc>>, Failure> {
    let now = Utc::now();
    let mut transaction = pool.begin().await?;
    for alarm in store::alarms::due(&mut transaction, now).await? {
        // Dropped when it cannot be sent: the alarm is settled all the
        // same.
        let _ = state.daemon.wake(&state.client, &alarm.agent, &alarm.note).await;
        let next = alarm.cron.as_deref().and_then(|cron| schedule::after(cron, now));
        store::alarms::settle(&mut transaction, alarm.id, next).await?;
    }
    transaction.commit().await?;
    store::alarms::next(pool).await
}
