//! The clock: a service nobody is attached to, retired.

use std::sync::Arc;

use super::{State, retire};
use crate::daemon::Daemon;
use crate::store::DependencyToolId;

/// Run the service's clock until it is retired: while any agent is
/// attached the clock does not run; otherwise, `idle_seconds` after
/// the attachments last changed with no change since, the service is
/// retired. The countdown begins only when nothing is attached, and
/// begins again from the last change whenever that becomes so, so an
/// agent attaching and leaving inside the window starts it over.
pub async fn clock(daemon: Arc<Daemon>, id: DependencyToolId, state: Arc<State>) {
    let mut held = state.held.subscribe();
    let mut touched = state.touched.subscribe();
    loop {
        if *held.borrow_and_update() {
            if held.changed().await.is_err() {
                return;
            }
            continue;
        }
        let last = *touched.borrow_and_update();
        let deadline = last + daemon.idle;
        tokio::select! {
            () = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                if !*held.borrow() && *touched.borrow() == last {
                    retire(&daemon, id).await;
                    return;
                }
            }
            changed = touched.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            changed = held.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}
