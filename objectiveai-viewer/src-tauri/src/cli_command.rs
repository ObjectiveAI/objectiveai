//! Tauri command `cli_execute` — spawns the objectiveai cli binary
//! via the SDK's [`BinaryExecutor`] and forwards each stdout JSONL
//! line to the iframe that invoked the cli as an
//! [`Event::CliCommand`](objectiveai_sdk::viewer::Event) event,
//! terminated by a synthetic `{"type":"end"}` marker.
//!
//! `cli_execute` takes a typed
//! [`Request`](objectiveai_sdk::cli::command::Request) as serde JSON
//! and lowers it to argv via `into_command()` — the canonical
//! Request→argv mapping lives in Rust, never in JS, and there is
//! deliberately NO raw-argv Tauri command (the argv-level
//! [`cli_run_impl`] is internal plumbing + test surface only).
//!
//! The plugin-bridge resolves the originating iframe (via
//! `MessageEvent.source`) and passes its repository name as `origin`,
//! which becomes the `destination` on every emitted event. The plugin
//! never sets `destination` itself.

use futures::StreamExt;
use objectiveai_sdk::cli::command::binary::{BinaryExecutor, Error as BinaryError};
use objectiveai_sdk::cli::command::{AgentArguments, CommandExecutor};
use objectiveai_sdk::viewer::{Event, EventSender};

/// Per-call identity stamped on every cli child the viewer spawns:
/// instance hierarchy `"Viewer"`, every other field `None` so the
/// executor `env_remove`s it and nothing leaks from the viewer's own
/// environment.
pub(crate) fn viewer_agent_arguments() -> AgentArguments {
    AgentArguments {
        agent_instance_hierarchy: Some("Viewer".to_string()),
        ..AgentArguments::default()
    }
}

/// Run a typed [`Request`](objectiveai_sdk::cli::command::Request)
/// (sent by the JS SDK's generated viewer execute functions as serde
/// JSON). Deserializes it and lowers it to argv via `into_command()`,
/// then runs the cli binary through [`cli_run_impl`]. A request that
/// doesn't deserialize still resolves the iframe's iterator: one
/// error envelope, then the end marker.
///
/// Returns immediately after spawning the child + forwarder task; the
/// iframe sees output asynchronously via the events channel. When the
/// child's stdout closes, a final `{"type":"end"}` event is emitted —
/// the JS async iterator terminates only on that marker.
#[tauri::command]
pub async fn cli_execute(
    executor: tauri::State<'_, BinaryExecutor>,
    events_tx: tauri::State<'_, EventSender>,
    request: serde_json::Value,
    origin: String,
) -> Result<(), String> {
    cli_execute_impl(executor.inner(), events_tx.inner().clone(), request, origin).await
}

/// Tauri-free body of [`cli_execute`], mirroring [`cli_run_impl`].
#[doc(hidden)]
pub async fn cli_execute_impl(
    executor: &BinaryExecutor,
    events_tx: EventSender,
    request: serde_json::Value,
    origin: String,
) -> Result<(), String> {
    let request: objectiveai_sdk::cli::command::Request =
        match serde_json::from_value(request) {
            Ok(request) => request,
            Err(e) => {
                let _ = events_tx.send(Event::CliCommand {
                    destination: origin.clone(),
                    value: serde_json::json!({
                        "type": "error",
                        "level": "error",
                        "fatal": true,
                        "message": format!("request did not deserialize: {e}"),
                    }),
                });
                let _ = events_tx.send(Event::CliCommand {
                    destination: origin,
                    value: serde_json::json!({ "type": "end" }),
                });
                return Ok(());
            }
        };
    cli_run_impl(executor, events_tx, request, origin).await
}

/// Run core shared by [`cli_execute`] (which deserializes the JSON
/// request first) and the integration tests (which construct a
/// `BinaryExecutor::from_path(...)` aimed at a test-built cli). The
/// `BinaryExecutor` serializes the request and invokes the cli as
/// `objectiveai --request <json>`. NOT exposed as a Tauri command.
#[doc(hidden)]
pub async fn cli_run_impl(
    executor: &BinaryExecutor,
    events_tx: EventSender,
    request: objectiveai_sdk::cli::command::Request,
    origin: String,
) -> Result<(), String> {
    // Spawn the child before detaching the forwarder so the executor
    // borrow doesn't have to live inside the 'static task.
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
                        destination: origin.clone(),
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
                    destination: origin.clone(),
                    value: error_value(e),
                });
            }
        }
        // The JS invokeCli iterator terminates ONLY on this marker —
        // always emit it last, even after an error, so the iframe's
        // async iterator never hangs.
        let _ = events_tx.send(Event::CliCommand {
            destination: origin,
            value: serde_json::json!({ "type": "end" }),
        });
    });
    Ok(())
}

/// Project a [`BinaryError`] onto the cli's JSONL error envelope. A
/// `Cli` error IS the envelope the cli printed (`type:"error"`) — it
/// re-serializes unchanged; everything else (spawn / io / decode) is
/// synthesized into the same wire shape.
fn error_value(e: BinaryError) -> serde_json::Value {
    match e {
        BinaryError::Cli(err) => serde_json::to_value(&err).unwrap_or_else(|_| {
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
