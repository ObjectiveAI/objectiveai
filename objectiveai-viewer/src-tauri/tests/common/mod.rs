//! Shared harness for the viewer's integration tests.
//!
//! Each test builds a [`ViewerTestEnv`] which provides:
//!
//! - A real `mpsc::unbounded_channel` `EventSender` / `EventReceiver`
//!   pair — events flow through here as they would in production.
//! - A real `objectiveai_sdk::HttpClient` pointed at the shared test
//!   API server. The server is the same one that the workspace's
//!   `test.sh` spawns (`test-spawn-api-server.sh`) — it binds to a
//!   free port with `MOCK_DELAY_MS=0` and serves mock-agent responses
//!   deterministically per seed. Address comes in via the
//!   `OBJECTIVEAI_TEST_PORT` env var; tests skip if it's unset.
//! - Drain helpers that collect events from the receiver until a
//!   user-supplied predicate matches or a timeout fires.
//!
//! The harness deliberately skips the Tauri builder. Integration
//! tests drive `cli_run_impl` / `api_call_run_impl` (exposed via the
//! viewer crate's `test_internals` module) directly and assert on
//! the events that land on the channel.

#![allow(dead_code)]

pub mod snapshot;

use std::time::Duration;

use objectiveai_sdk::HttpClient;
use objectiveai_sdk::viewer::{Event, EventReceiver, EventSender};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Reads `OBJECTIVEAI_TEST_PORT` and returns the URL pointing at the
/// shared test API server. Skips the test (via `eprintln!` + `return
/// Ok(())` from the caller) if the env var isn't set — used so a
/// developer running `cargo test -p objectiveai-viewer` without the
/// orchestrator's env doesn't get spurious failures.
pub fn test_api_address() -> Option<String> {
    let port = std::env::var("OBJECTIVEAI_TEST_PORT").ok()?;
    Some(format!("http://127.0.0.1:{port}"))
}

pub struct ViewerTestEnv {
    pub events_tx: EventSender,
    pub events_rx: EventReceiver,
    pub http_client: HttpClient,
}

impl ViewerTestEnv {
    /// Build a minimal viewer. Panics if `OBJECTIVEAI_TEST_PORT` is
    /// unset — caller should `if test_api_address().is_none() {
    /// return; }` first to skip gracefully outside the orchestrator.
    pub fn new() -> Self {
        let address = test_api_address()
            .expect("OBJECTIVEAI_TEST_PORT must be set; run via the workspace test.sh");
        let (events_tx, events_rx) = mpsc::unbounded_channel::<Event>();
        let none = || -> Option<String> { None };
        let http_client = HttpClient::new(
            reqwest::Client::new(),
            Some(address),
            none(), // authorization
            none(), // user_agent
            none(), // x_title
            none(), // http_referer
            none(), // x_github_authorization
            none(), // x_openrouter_authorization
            None,   // x_mcp_authorization
            none(), // x_viewer_signature
            none(), // x_viewer_address
            none(), // x_commit_author_name
            none(), // x_commit_author_email
        );
        Self {
            events_tx,
            events_rx,
            http_client,
        }
    }

    /// Drain `events_rx` until `is_end(event) == true` for one of
    /// the received events, then return everything collected so far
    /// (inclusive of the terminator). Times out and panics if the
    /// terminator doesn't arrive within `deadline` — keeps a stuck
    /// test from hanging the suite.
    pub async fn drain_until_end<F>(
        &mut self,
        is_end: F,
        deadline: Duration,
    ) -> Vec<Event>
    where
        F: Fn(&Event) -> bool,
    {
        let rx = &mut self.events_rx;
        let collect = async move {
            let mut events: Vec<Event> = Vec::new();
            while let Some(event) = rx.recv().await {
                let end = is_end(&event);
                events.push(event);
                if end {
                    break;
                }
            }
            events
        };
        match timeout(deadline, collect).await {
            Ok(events) => events,
            Err(_) => panic!(
                "drain_until_end timed out after {deadline:?} without receiving an end marker"
            ),
        }
    }
}

/// True when `event` is an `Event::ApiCall` whose value is the
/// `{"type":"end"}` envelope — terminator for an api_call flow.
pub fn is_api_call_end(event: &Event) -> bool {
    matches!(
        event,
        Event::ApiCall { value, .. }
            if value.get("type").and_then(|t| t.as_str()) == Some("end")
    )
}

/// True when `event` is an `Event::CliCommand` whose value is the
/// `{"type":"end"}` cli output line — terminator for a cli_command
/// flow.
pub fn is_cli_command_end(event: &Event) -> bool {
    matches!(
        event,
        Event::CliCommand { value, .. }
            if value.get("type").and_then(|t| t.as_str()) == Some("end")
    )
}
