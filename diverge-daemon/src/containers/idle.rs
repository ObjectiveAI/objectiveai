//! The idle clock: an unused container stopped.

use std::sync::Arc;

use super::AgentRun;
use crate::daemon::Daemon;

/// Run the agent's idle clock until the run ends: while a loop runs
/// the clock does not; otherwise, `idle_seconds` after the last use
/// with no use since, the run is stopped. The pump hears the end and
/// does the rest.
pub async fn idle(daemon: Arc<Daemon>, run: Arc<AgentRun>) {
    let mut active = run.loop_active.subscribe();
    let mut touched = run.touched.subscribe();
    loop {
        if *active.borrow() {
            if active.changed().await.is_err() {
                return;
            }
            continue;
        }
        let last = *touched.borrow_and_update();
        let deadline = last + daemon.idle;
        tokio::select! {
            () = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                if !*active.borrow() && *touched.borrow() == last {
                    let _ = run.handle.stop().await;
                    return;
                }
            }
            changed = touched.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            changed = active.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}
