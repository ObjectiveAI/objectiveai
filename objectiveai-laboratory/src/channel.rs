//! One daemon channel: dial `<daemon>/laboratory`, attach, then serve
//! [`ChannelRequest`]s until the socket drops — forever, with a 1s
//! reconnect pause, so a daemon restart just re-registers us. The host
//! runs one `run` task per configured daemon address, all sharing one
//! [`HostServer`].
//!
//! Wire order is load-bearing: the FIRST text frame is the
//! `HostIdentify` (who this HOST is — state, machine identity, and its
//! FULL current laboratory set), the SECOND is the authorization
//! envelope (`{"signature": …}` — the daemon's standard first-message
//! auth, demoted to second place here because identity always precedes
//! authorization on this endpoint). Both are enqueued by
//! [`HostServer::attach_channel`] onto the SAME single-writer queue
//! the replies and notifications use, so that order — and the
//! atomicity of the identify snapshot against concurrent
//! create/delete broadcasts — holds by construction; this loop never
//! writes a frame itself.

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use objectiveai_sdk::client_objectiveai_mcp::laboratory::ChannelRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::host::HostServer;

pub async fn run(
    daemon_address: String,
    signature: Option<String>,
    host: Arc<HostServer>,
    suppress_output: bool,
) {
    let url = format!("{}/laboratory", daemon_address.trim_end_matches('/'));
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
                // ALL outbound frames funnel through one writer task;
                // each request is served concurrently.
                let (reply_tx, mut reply_rx) =
                    tokio::sync::mpsc::unbounded_channel::<String>();
                let writer = tokio::spawn(async move {
                    while let Some(frame) = reply_rx.recv().await {
                        if sink.send(Message::Text(frame)).await.is_err() {
                            break;
                        }
                    }
                });
                // Attach: enqueues identify + auth as the first two
                // frames and subscribes this channel to notifications,
                // atomically against create/delete broadcasts.
                let channel_id =
                    host.attach_channel(reply_tx.clone(), auth_frame.clone()).await;
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
                    let host = Arc::clone(&host);
                    let reply_tx = reply_tx.clone();
                    tokio::spawn(async move {
                        let response = host.handle(request).await;
                        if let Ok(frame) = serde_json::to_string(&response) {
                            let _ = reply_tx.send(frame);
                        }
                    });
                }
                host.detach_channel(channel_id);
                drop(reply_tx);
                let _ = writer.await;
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
