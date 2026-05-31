//! Shared harness for the viewer's integration tests.
//!
//! Each test builds a [`ViewerTestEnv`] which provides:
//!
//! - A real `mpsc::unbounded_channel` `EventSender` / `EventReceiver`
//!   pair — events flow through here as they would in production.
//! - Drain helpers that collect events from the receiver until a
//!   user-supplied predicate matches or a timeout fires.
//!
//! The harness deliberately skips the Tauri builder. Integration
//! tests drive `cli_run_impl` (exposed via the viewer crate's
//! `test_internals` module) directly and assert on the events that
//! land on the channel.

#![allow(dead_code)]

pub mod snapshot;

use std::time::Duration;

use objectiveai_sdk::viewer::{Event, EventReceiver, EventSender};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Reads `OBJECTIVEAI_TEST_PORT` and returns the URL pointing at the
/// shared test API server. Skips the test (via `eprintln!` + early
/// return from the caller) if the env var isn't set — used so a
/// developer running `cargo test -p objectiveai-viewer` without the
/// orchestrator's env doesn't get spurious failures.
pub fn test_api_address() -> Option<String> {
    let port = std::env::var("OBJECTIVEAI_TEST_PORT").ok()?;
    Some(format!("http://127.0.0.1:{port}"))
}

pub struct ViewerTestEnv {
    pub events_tx: EventSender,
    pub events_rx: EventReceiver,
}

impl ViewerTestEnv {
    /// Build a minimal viewer-events channel pair for driving the
    /// cli_command bridge in tests.
    pub fn new() -> Self {
        let (events_tx, events_rx) = mpsc::unbounded_channel::<Event>();
        Self { events_tx, events_rx }
    }

    /// Drain `events_rx` until the channel closes (every sender
    /// clone dropped) and return everything collected. Drops the
    /// harness's own sender first so the only remaining clone is
    /// whichever one the cli task is holding — once the cli
    /// finishes and its handle drops, recv returns `None`.
    /// Times out and panics if recv doesn't close within
    /// `deadline`.
    pub async fn drain_until_close(mut self, deadline: Duration) -> Vec<Event> {
        drop(self.events_tx);
        let collect = async {
            let mut events: Vec<Event> = Vec::new();
            while let Some(event) = self.events_rx.recv().await {
                events.push(event);
            }
            events
        };
        match timeout(deadline, collect).await {
            Ok(events) => events,
            Err(_) => panic!(
                "drain_until_close timed out after {deadline:?} without the channel closing"
            ),
        }
    }
}
