//! Tauri command `cli_run` — spawns the objectiveai cli binary via the
//! SDK's [`BinaryExecutor`] and forwards each stdout JSONL line to the
//! iframe that invoked the cli as an
//! [`Event::CliCommand`](objectiveai_sdk::viewer::Event) event,
//! terminated by a synthetic `{"type":"end"}` marker.
//!
//! The plugin-bridge resolves the originating iframe (via
//! `MessageEvent.source`) and passes its repository name as `origin`,
//! which becomes the `destination` on every emitted event. The plugin
//! never sets `destination` itself.

use futures::StreamExt;
use objectiveai_sdk::cli::command::binary::{BinaryExecutor, Error as BinaryError};
use objectiveai_sdk::cli::command::{AgentArguments, CommandExecutor, CommandRequest};
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

/// Verbatim argv passthrough. The cli strips its own argv[0] and its
/// `parse_request` tolerates an optional leading literal
/// `"objectiveai"`, so whatever shape the iframe sent works unchanged.
struct RawArgs(Vec<String>);

impl CommandRequest for RawArgs {
    fn into_command(&self) -> Vec<String> {
        self.0.clone()
    }
}

/// Run the cli binary with `args`. Each emitted JSONL line is wrapped
/// as `Event::CliCommand { destination: origin, value }` and pushed
/// onto the viewer's events bus, where the JS bridge picks it up and
/// forwards to the originating iframe.
///
/// Returns immediately after spawning the child + forwarder task; the
/// iframe sees output asynchronously via the events channel. When the
/// child's stdout closes, a final `{"type":"end"}` event is emitted —
/// the JS `invokeCli` async iterator terminates only on that marker.
#[tauri::command]
pub async fn cli_run(
    executor: tauri::State<'_, BinaryExecutor>,
    events_tx: tauri::State<'_, EventSender>,
    args: Vec<String>,
    origin: String,
) -> Result<(), String> {
    cli_run_impl(executor.inner(), events_tx.inner().clone(), args, origin).await
}

/// Tauri-free body of [`cli_run`]. Lets integration tests exercise
/// the bridge without constructing a `tauri::State` — they pass a
/// `BinaryExecutor::from_path(...)` aimed at a test-built cli. Same
/// fire-and-forget semantics as the Tauri-wrapped form.
#[doc(hidden)]
pub async fn cli_run_impl(
    executor: &BinaryExecutor,
    events_tx: EventSender,
    args: Vec<String>,
    origin: String,
) -> Result<(), String> {
    // Spawn the child before detaching the forwarder so the executor
    // borrow doesn't have to live inside the 'static task.
    let agent_arguments = viewer_agent_arguments();
    let stream = executor
        .execute::<RawArgs, serde_json::Value>(RawArgs(args), Some(&agent_arguments))
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
