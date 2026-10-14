//! The clock: a service nothing holds, retired.

use std::sync::Arc;

use super::{State, retire};
use crate::containers::ToolKey;
use crate::daemon::Daemon;

/// Run the service's clock until it is retired: while anything holds
/// the service the clock does not run; otherwise, `idle_seconds` after
/// the hold last changed with no change since, the service is
/// retired. The countdown begins only when nothing holds it, and
/// begins again from the last change whenever that becomes so, so an
/// agent attaching and leaving inside the window, or a connect
/// opening and closing, starts it over.
pub async fn clock(daemon: Arc<Daemon>, key: ToolKey, state: Arc<State>) {
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
                    retire(&daemon, key).await;
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
