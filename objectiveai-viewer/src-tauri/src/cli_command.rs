//! Tauri command `cli_execute` — runs a CLI command against the
//! daemon over WebSocket via the SDK's [`WebSocketExecutor`] and
//! forwards each stream item to the iframe that invoked the cli as an
//! [`Event::CliCommand`](objectiveai_sdk::viewer::Event) event,
//! terminated by a synthetic `{"type":"end"}` marker.
//!
//! The viewer never spawns the cli binary: commands travel to the
//! daemon's `/execute` route and run in-process there, which is what
//! lets the viewer live on a different machine than the CLI.
//!
//! `cli_execute` takes a typed
//! [`Request`](objectiveai_sdk::cli::command::Request) as serde JSON —
//! the canonical request shape lives in Rust, never in JS, and there
//! is deliberately NO raw-argv Tauri command (the request-level
//! [`cli_run_impl`] is internal plumbing + test surface only).
//!
//! Whoever runs a request gets its own response back: the caller
//! passes its [`Destination`] (the main UI, or a plugin's full
//! coordinates — the plugin-bridge resolves those from
//! `MessageEvent.source`; a plugin never claims an identity itself),
//! and every emitted event carries it.

use futures::StreamExt;
use objectiveai_sdk::cli::command::websocket::{Error as WsError, WebSocketExecutor};
use objectiveai_sdk::cli::command::{AgentArguments, CommandExecutor};
use objectiveai_sdk::viewer::{Destination, Event, EventSender};

/// Per-call identity sent in every execute envelope: instance
/// hierarchy `"Viewer"`, every other field `None` so the daemon
/// clears rather than inherits it — nothing leaks from the daemon's
/// own environment into a viewer-initiated run.
pub(crate) fn viewer_agent_arguments() -> AgentArguments {
    AgentArguments {
        agent_instance_hierarchy: Some("Viewer".to_string()),
        ..AgentArguments::default()
    }
}

/// Run a typed [`Request`](objectiveai_sdk::cli::command::Request)
/// (sent by the JS SDK's generated viewer execute functions as serde
/// JSON) through the daemon's `/execute` route. A request that
/// doesn't deserialize still resolves the iframe's iterator: one
/// error envelope, then the end marker.
///
/// Returns immediately after opening the connection + forwarder task;
/// the iframe sees output asynchronously via the events channel. When
/// the daemon closes the stream, a final `{"type":"end"}` event is
/// emitted — the JS async iterator terminates only on that marker.
#[tauri::command]
pub async fn cli_execute(
    executor: tauri::State<'_, WebSocketExecutor>,
    events_tx: tauri::State<'_, EventSender>,
    request: serde_json::Value,
    destination: Destination,
) -> Result<(), String> {
    cli_execute_impl(
        executor.inner(),
        events_tx.inner().clone(),
        request,
        destination,
    )
    .await
}

/// Tauri-free body of [`cli_execute`], mirroring [`cli_run_impl`].
#[doc(hidden)]
pub async fn cli_execute_impl(
    executor: &WebSocketExecutor,
    events_tx: EventSender,
    request: serde_json::Value,
    destination: Destination,
) -> Result<(), String> {
    let request: objectiveai_sdk::cli::command::Request =
        match serde_json::from_value(request) {
            Ok(request) => request,
            Err(e) => {
                let _ = events_tx.send(Event::CliCommand {
                    destination: destination.clone(),
                    value: serde_json::json!({
                        "type": "error",
                        "level": "error",
                        "fatal": true,
                        "message": format!("request did not deserialize: {e}"),
                    }),
                });
                let _ = events_tx.send(Event::CliCommand {
                    destination,
                    value: serde_json::json!({ "type": "end" }),
                });
                return Ok(());
            }
        };
    cli_run_impl(executor, events_tx, request, destination).await
}

/// Run core shared by [`cli_execute`] (which deserializes the JSON
/// request first) and the integration tests (which aim a
/// `WebSocketExecutor` at a test daemon). The executor sends the
/// request in its execute envelope to the daemon's `/execute` route.
/// NOT exposed as a Tauri command.
#[doc(hidden)]
pub async fn cli_run_impl(
    executor: &WebSocketExecutor,
    events_tx: EventSender,
    request: objectiveai_sdk::cli::command::Request,
    destination: Destination,
) -> Result<(), String> {
    // Open the connection before detaching the forwarder so the
    // executor borrow doesn't have to live inside the 'static task.
    let agent_arguments = viewer_agent_arguments();
    let stream = executor
        .execute::<objectiveai_sdk::cli::command::Request, serde_json::Value>(
            request,
            Some(&agent_arguments),
        )
        .await;
    tokio::spawn(async move {
        match stream {
            Ok(mut stream) => {
                while let Some(item) = stream.next().await {
                    let value = match item {
                        Ok(value) => value,
                        Err(e) => error_value(e),
                    };
                    let event = Event::CliCommand {
                        destination: destination.clone(),
                        value,
                    };
                    if events_tx.send(event).is_err() {
                        // Events bus is gone — the viewer is shutting
                        // down; nothing left to forward to.
                        return;
                    }
                }
            }
            Err(e) => {
                let _ = events_tx.send(Event::CliCommand {
                    destination: destination.clone(),
                    value: error_value(e),
                });
            }
        }
        // The JS invokeCli iterator terminates ONLY on this marker —
        // always emit it last, even after an error, so the iframe's
        // async iterator never hangs.
        let _ = events_tx.send(Event::CliCommand {
            destination,
            value: serde_json::json!({ "type": "end" }),
        });
    });
    Ok(())
}

/// Project a [`WsError`] onto the cli's JSONL error envelope. A `Cli`
/// error IS the envelope the daemon sent (`type:"error"`) — it
/// re-serializes unchanged; everything else (connect / transport /
/// decode) is synthesized into the same wire shape.
fn error_value(e: WsError) -> serde_json::Value {
    match e {
        WsError::Cli(err) => serde_json::to_value(&err).unwrap_or_else(|_| {
            serde_json::json!({
                "type": "error",
                "level": "error",
                "fatal": true,
                "message": "cli error failed to re-serialize",
            })
        }),
        other => serde_json::json!({
            "type": "error",
            "level": "error",
            "fatal": true,
            "message": format!("{other:?}"),
        }),
    }
}
