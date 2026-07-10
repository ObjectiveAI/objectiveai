//! The daemon channel: dial `<daemon>/laboratory`, identify, authorize,
//! then serve [`ChannelRequest`]s until the socket drops — forever, with
//! a 1s reconnect pause, so a daemon restart just re-registers us.
//!
//! Wire order is load-bearing: the FIRST text frame is the
//! [`Identify`] (who this laboratory is), the SECOND is the
//! authorization envelope (`{"signature": …}` — the daemon's standard
//! first-message auth, demoted to second place here because identity
//! always precedes authorization on this endpoint).

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{ChannelRequest, Identify};
use tokio_tungstenite::tungstenite::Message;

use crate::server::LabServer;

pub async fn run(
    daemon_address: String,
    identify: Identify,
    signature: Option<String>,
    server: Arc<LabServer>,
    suppress_output: bool,
    on_first_connect: Box<dyn FnOnce() + Send>,
) {
    let mut on_first_connect = Some(on_first_connect);
    let url = format!("{}/laboratory", daemon_address.trim_end_matches('/'));
    let identify_frame =
        serde_json::to_string(&identify).expect("identify serializes");
    // Mirrors the SDK `AuthEnvelope` shape without pulling the cli
    // feature in for one two-field struct.
    let auth_frame = serde_json::json!({ "signature": signature }).to_string();

    loop {
        match tokio_tungstenite::connect_async(&url).await {
            Ok((ws, _)) => {
                if !suppress_output {
                    eprintln!("connected: {url}");
                }
                let (mut sink, mut stream) = ws.split();
                if sink.send(Message::Text(identify_frame.clone())).await.is_err()
                    || sink.send(Message::Text(auth_frame.clone())).await.is_err()
                {
                    // Fall through to the reconnect pause.
                } else {
                    // First successful connection: fire the one-shot
                    // hook (spawns the cleaner sweep) — strictly after
                    // the WS is up so it delays nothing.
                    if let Some(hook) = on_first_connect.take() {
                        hook();
                    }
                    // Replies funnel through one writer task; each request
                    // is served concurrently.
                    let (reply_tx, mut reply_rx) =
                        tokio::sync::mpsc::unbounded_channel::<String>();
                    let writer = tokio::spawn(async move {
                        while let Some(frame) = reply_rx.recv().await {
                            if sink.send(Message::Text(frame)).await.is_err() {
                                break;
                            }
                        }
                    });
                    while let Some(frame) = stream.next().await {
                        let text = match frame {
                            Ok(Message::Text(text)) => text,
                            Ok(Message::Close(_)) | Err(_) => break,
                            // Pings are answered by tungstenite itself;
                            // ignore everything else.
                            Ok(_) => continue,
                        };
                        let Ok(request) =
                            serde_json::from_str::<ChannelRequest>(&text)
                        else {
                            // Forward-compat: skip frames this build
                            // doesn't know.
                            continue;
                        };
                        let server = Arc::clone(&server);
                        let reply_tx = reply_tx.clone();
                        tokio::spawn(async move {
                            let response = server.handle(request).await;
                            if let Ok(frame) = serde_json::to_string(&response) {
                                let _ = reply_tx.send(frame);
                            }
                        });
                    }
                    drop(reply_tx);
                    let _ = writer.await;
                }
                if !suppress_output {
                    eprintln!("disconnected: {url}; retrying");
                }
            }
            Err(e) => {
                if !suppress_output {
                    eprintln!("connect {url}: {e}; retrying");
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
