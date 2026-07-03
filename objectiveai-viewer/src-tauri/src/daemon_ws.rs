//! Daemon WebSocket client — the viewer's one data source.
//!
//! The viewer is not a server: it consumes the CLI daemon's broadcast
//! WebSocket, which carries every CLI run as a `RootViewerRequest`
//! frame (`{…context, id, value}`) followed by `RootViewerResponseItem`
//! frames (`{id, path_type, value}`). Every frame is forwarded raw to
//! the JS side as `Event::Inbound { destination: "objectiveai",
//! sub_type: "daemon", value: <frame> }` — the frontend discriminates
//! and routes (e.g. `plugins/run` frames to the matching plugin tab).
//!
//! Connection lifecycle, per loop iteration:
//! 1. Ensure the daemon is up: drive `objectiveai daemon spawn`
//!    through the [`BinaryExecutor`] (idempotent — a no-op when the
//!    daemon already holds its lock).
//! 2. Read the daemon's lock for its published `ws://` URL. The daemon
//!    binds its listeners BEFORE publishing, so a present lock means
//!    the endpoint is connectable.
//! 3. Connect, attaching `X-DAEMON-SIGNATURE` when the viewer was
//!    started with a `DAEMON_SIGNATURE` (the pre-derived
//!    `sha256=<hex(SHA256(DAEMON_SECRET))>` — optional; without it the
//!    daemon must be running unauthenticated).
//! 4. Pump frames until the connection drops, then sleep briefly and
//!    start over — the viewer stays live across daemon restarts.

use std::path::PathBuf;

use futures::StreamExt;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use objectiveai_sdk::cli::command::daemon::spawn as daemon_spawn;
use objectiveai_sdk::viewer::{Event, EventSender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

/// Lock key the daemon publishes its `ws://` URL under — mirrors
/// `objectiveai-cli`'s `command::daemon::DAEMON_LOCK_KEY` (the viewer
/// cannot import the cli crate, so the constant is duplicated here).
const DAEMON_LOCK_KEY: &str = "plugins-daemon";

/// Spawn the resident client task. Best-effort forever-loop: any
/// failure (spawn, lock read, connect, mid-stream drop) falls through
/// to a short sleep and a fresh attempt. Exits only when the event bus
/// receiver is gone (the viewer is shutting down).
pub(crate) fn spawn_client(
    tx: EventSender,
    executor: BinaryExecutor,
    lock_dir: PathBuf,
    signature: Option<String>,
) {
    tokio::spawn(async move {
        loop {
            ensure_daemon(&executor).await;

            if let Ok(Some(url)) =
                objectiveai_sdk::lockfile::try_read(&lock_dir, DAEMON_LOCK_KEY).await
            {
                if pump(&tx, &url, signature.as_deref()).await.is_err() {
                    // Receiver gone: the viewer is shutting down.
                    return;
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

/// Drive `daemon spawn` to completion through the cli binary.
/// Idempotent on the daemon side; errors are swallowed (the connect
/// below is the real health check).
async fn ensure_daemon(executor: &BinaryExecutor) {
    let request = daemon_spawn::Request {
        path_type: daemon_spawn::Path::DaemonSpawn,
        dangerous_advanced: None,
        base: Default::default(),
    };
    let agent_arguments = crate::cli_command::viewer_agent_arguments();
    if let Ok(mut stream) =
        daemon_spawn::execute(executor, request, Some(&agent_arguments)).await
    {
        while let Some(_item) = stream.next().await {}
    }
}

/// One connection: upgrade (with the optional signature header) and
/// forward every text frame to the event bus until the socket drops.
/// `Err(())` means the event bus is closed — stop entirely.
async fn pump(tx: &EventSender, url: &str, signature: Option<&str>) -> Result<(), ()> {
    let Ok(mut request) = url.into_client_request() else {
        return Ok(());
    };
    if let Some(signature) = signature {
        if let Ok(value) = HeaderValue::from_str(signature) {
            request.headers_mut().insert("X-DAEMON-SIGNATURE", value);
        }
    }
    let Ok((mut ws, _response)) = connect_async(request).await else {
        return Ok(());
    };
    while let Some(message) = ws.next().await {
        match message {
            Ok(Message::Text(text)) => {
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                if tx
                    .send(Event::Inbound {
                        destination: "objectiveai".to_string(),
                        sub_type: "daemon".to_string(),
                        value,
                    })
                    .is_err()
                {
                    return Err(());
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(_) => {}
        }
    }
    Ok(())
}
