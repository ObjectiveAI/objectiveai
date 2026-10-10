//! The idle clock: an unused container stopped.

use std::sync::Arc;

use super::{AgentRun, ToolRun};
use crate::daemon::Daemon;

/// Run the agent's idle clock until the run ends: while the agent is
/// active — a loop runs in it, or an MCP exchange is in flight for it
/// or one of its dependencies — the clock does not run; otherwise,
/// `idle_seconds` after the last use with no use since, the run is
/// stopped. The countdown begins only when both are false, and begins
/// again from the last touch whenever both become false. The pump
/// hears the end and does the rest.
pub async fn idle(daemon: Arc<Daemon>, run: Arc<AgentRun>) {
    let mut active = run.loop_active.subscribe();
    let mut inflight = run.inflight.subscribe();
    let mut touched = run.touched.subscribe();
    loop {
        if *active.borrow() || *inflight.borrow() > 0 {
            tokio::select! {
                changed = active.changed() => {
                    if changed.is_err() {
                        return;
                    }
                }
                changed = inflight.changed() => {
                    if changed.is_err() {
                        return;
                    }
                }
            }
            continue;
        }
        let last = *touched.borrow_and_update();
        let deadline = last + daemon.idle;
        tokio::select! {
            () = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                if !*active.borrow() && *inflight.borrow() == 0 && *touched.borrow() == last {
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
            changed = inflight.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}

/// Run a connected tool's idle clock until the run ends: while
/// anything uses the tool — an attached agent's call, a file operation
/// — the clock does not run; otherwise, `idle_seconds` after the last
/// use with no use since, the connection to the other daemon is let
/// go. The countdown begins only when nothing uses it, and begins
/// again from the last touch whenever that becomes so. The waiter
/// hears the end and does the rest. A container the daemon runs has
/// no clock: it is stopped by the last user leaving.
pub async fn idle_tool(daemon: Arc<Daemon>, run: Arc<ToolRun>) {
    let mut held = run.held.subscribe();
    let mut touched = run.touched.subscribe();
    loop {
        if *held.borrow() {
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
                    run.handle.stop().await;
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
