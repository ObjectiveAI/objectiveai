//! Daemon `/listen` client — the main viewer UI's data source, built
//! on the SDK's typed
//! [`WebSocketListener`](objectiveai_sdk::cli::websocket_listener::WebSocketListener).
//!
//! The viewer is not a server: it consumes the CLI daemon's broadcast
//! through the SDK listener and is a pure PASSTHROUGH — each typed
//! `ListenerExecution` the listener yields is flattened back to
//! wire-shaped JSON ([`crate::serialize::into_serialized`]) and
//! re-packaged into the same standard three-frame envelope the daemon
//! broadcasts, emitted to the JS side as `Event::Inbound {
//! destination: Objectiveai, value: <frame> }`:
//!
//!   - request:    `{…agent arguments, id, value: <request>}`
//!   - response:   `{id, value: <item>}` per item (in-band
//!     `cli::Error` lines re-serialize exactly as they arrived)
//!   - terminator: `{id, end: true}` when the run's response ends
//!
//! Frame `id`s are minted fresh per run (the typed envelope doesn't
//! carry the daemon's broadcast id; consumers only need per-run
//! consistency). Every frame from THIS client is destined to the main
//! viewer UI only; plugin delivery of `plugins/run` runs is the
//! separate [`crate::viewer_plugin_listener`] client's job — its own
//! daemon connection, fully independent of this one (the daemon
//! broadcasts to every connected socket).
//!
//! The daemon's base `ws://` URL arrives via the REQUIRED
//! `DAEMON_ADDRESS` env (set by `objectiveai viewer spawn`, which
//! ensures the daemon is running before spawning the viewer) — the
//! viewer does no daemon discovery of its own. Auth is the optional
//! `DAEMON_SIGNATURE` (the pre-derived
//! `sha256=<hex(SHA256(DAEMON_SECRET))>`), handed to the listener
//! verbatim.
//!
//! On disconnect the client sleeps briefly and reconnects to the SAME
//! address. Note: a daemon restart binds a fresh ephemeral port, so
//! reconnection can only succeed while the original daemon is alive —
//! restart the viewer (via `viewer spawn`) to pick up a new daemon.
//! Runs the SDK listener skips (a `path_type` its types predate) don't
//! reach the JS side — the passthrough is typed, not raw.

use futures::StreamExt;
use objectiveai_sdk::cli::command::ListenerExecution;
use objectiveai_sdk::cli::websocket_listener::WebSocketListener;
use objectiveai_sdk::viewer::{Destination, Event, EventSender};

use crate::serialize::{SerializedListenerResponse, into_serialized};

/// Spawn the resident client task. Best-effort forever-loop: any
/// failure (connect, mid-stream drop) falls through to a short sleep
/// and a fresh attempt against the same address. Exits only when the
/// event bus receiver is gone (the viewer is shutting down).
pub(crate) fn spawn_client(tx: EventSender, address: String, signature: Option<String>) {
    tokio::spawn(async move {
        loop {
            if pump(&tx, &address, signature.as_deref()).await.is_err() {
                // Receiver gone: the viewer is shutting down.
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

/// Connect one [`WebSocketListener`] to the daemon's `/listen` route.
/// Shared with [`crate::viewer_plugin_listener`] — each caller gets
/// its OWN connection (the daemon broadcasts to every socket), so the
/// two clients never share state or interfere.
pub(crate) async fn connect(
    url: &str,
    signature: Option<&str>,
) -> Result<
    WebSocketListener,
    objectiveai_sdk::cli::websocket_listener::Error,
> {
    let mut builder = WebSocketListener::new(format!("{url}/listen"));
    if let Some(signature) = signature {
        builder = builder.signature(signature);
    }
    builder.connect().await
}

/// One connection: run the typed listener until its stream ends,
/// re-packaging every run for the main viewer UI. `Err(())` means the
/// event bus is closed — stop entirely.
async fn pump(tx: &EventSender, url: &str, signature: Option<&str>) -> Result<(), ()> {
    let Ok(mut listener) = connect(url, signature).await else {
        return Ok(());
    };
    while let Some(item) = listener.next().await {
        let Ok(execution) = item else {
            // Transport error: the listener's stream ends right after —
            // fall through to the reconnect loop.
            break;
        };
        emit_run(tx, execution, Destination::Objectiveai)?;
    }
    Ok(())
}

/// Re-package one run into the standard envelope for `destination`:
/// emit its request frame now, then spawn a task draining its
/// response into `{id, value}` frames and the final `{id, end: true}`
/// terminator. Shared with [`crate::viewer_plugin_listener`].
pub(crate) fn emit_run(
    tx: &EventSender,
    execution: ListenerExecution,
    destination: Destination,
) -> Result<(), ()> {
    let serialized = into_serialized(execution);
    let id = uuid::Uuid::new_v4().to_string();

    // Request frame: the agent arguments' fields flattened alongside
    // `id` and the serialized request — the daemon's own wrapper shape.
    let mut frame = match serde_json::to_value(&serialized.agent_arguments) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    frame.insert("id".to_string(), serde_json::json!(id.clone()));
    frame.insert("value".to_string(), serialized.request);
    send(tx, &destination, serde_json::Value::Object(frame))?;

    // Response frames + terminator, driven independently per run so a
    // slow stream never stalls the listener.
    let tx = tx.clone();
    tokio::spawn(async move {
        match serialized.response {
            SerializedListenerResponse::Unary(response) => {
                let value = match response.await {
                    Ok(value) => value,
                    Err(error) => {
                        serde_json::to_value(&error).unwrap_or(serde_json::Value::Null)
                    }
                };
                let _ = send(&tx, &destination, serde_json::json!({ "id": id, "value": value }));
            }
            SerializedListenerResponse::Stream(mut items) => {
                while let Some(item) = items.next().await {
                    let value = match item {
                        Ok(value) => value,
                        Err(error) => {
                            serde_json::to_value(&error).unwrap_or(serde_json::Value::Null)
                        }
                    };
                    if send(&tx, &destination, serde_json::json!({ "id": id, "value": value }))
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
        let _ = send(&tx, &destination, serde_json::json!({ "id": id, "end": true }));
    });
    Ok(())
}

/// One daemon frame onto the JS event bus.
fn send(tx: &EventSender, destination: &Destination, frame: serde_json::Value) -> Result<(), ()> {
    tx.send(Event::Inbound {
        destination: destination.clone(),
        value: frame,
    })
    .map_err(|_| ())
}
