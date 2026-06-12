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

/// `<repo>/.objectiveai` — the committed shared test
/// `OBJECTIVEAI_DIR` every integration test in the repository uses.
pub fn objectiveai_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a parent")
        .parent()
        .expect("crate dir has a parent")
        .join(".objectiveai")
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
