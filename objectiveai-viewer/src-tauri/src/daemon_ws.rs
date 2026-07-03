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
//! The daemon's `ws://` connect URL arrives via the REQUIRED
//! `DAEMON_ADDRESS` env (set by `objectiveai viewer spawn`, which
//! ensures the daemon is running before spawning the viewer) — the
//! viewer does no daemon discovery or spawning of its own. Auth is the
//! optional `DAEMON_SIGNATURE` (the pre-derived
//! `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent verbatim as
//! `X-DAEMON-SIGNATURE` on each upgrade.
//!
//! On disconnect the client sleeps briefly and reconnects to the SAME
//! address. Note: a daemon restart binds a fresh ephemeral port, so
//! reconnection can only succeed while the original daemon is alive —
//! restart the viewer (via `viewer spawn`) to pick up a new daemon.

use futures::StreamExt;
use objectiveai_sdk::viewer::{Event, EventSender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

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

/// One connection: upgrade (with the optional signature header) and
/// forward every text frame to the event bus until the socket drops.
/// `Err(())` means the event bus is closed — stop entirely.
async fn pump(tx: &EventSender, url: &str, signature: Option<&str>) -> Result<(), ()> {
    // The broadcast lives on the daemon's `/listen` route; `url` is
    // the published base ws:// address.
    let url = format!("{url}/listen");
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
