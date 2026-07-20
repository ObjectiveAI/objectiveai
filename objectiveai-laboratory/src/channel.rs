//! One daemon channel: dial `<daemon>/laboratory`, attach, then serve
//! [`ChannelRequest`]s until the socket drops — forever, with a 1s
//! reconnect pause, so a daemon restart just re-registers us — or
//! until the `cancel` watch fires (the stdin dial-list removed this
//! address), which tears down through the SAME detach path as a
//! natural disconnect and then ends the task instead of reconnecting.
//! The host runs one `run` task per stdin-added daemon address, all
//! sharing one [`HostServer`].
//!
//! Wire order is load-bearing: the FIRST text frame is the
//! `HostIdentify` (who this HOST is — state, machine identity, and its
//! FULL current laboratory set), the SECOND is the authorization
//! envelope (`{"signature": …}` — the daemon's standard first-message
//! auth, demoted to second place here because identity always precedes
//! authorization on this endpoint).
//!
//! Outbound frames ride TWO lanes, merged by one writer task per
//! channel:
//!
//! - **Control** (unbounded mpsc): identify, auth, attach-time
//!   synthesized filetree snapshots, correlated RPC responses, and the
//!   rare Created/Updated/Deleted notifications. Never dropped;
//!   request-paced, so effectively bounded.
//! - **Filetree** (bounded broadcast ring on the host): the per-event
//!   fire hose. A writer that falls behind gets `Lagged` and resyncs
//!   itself with fresh snapshots from
//!   [`HostServer::filetree_snapshot_frames`] — a stalled daemon
//!   socket costs a resync, never unbounded host memory (the same
//!   lag→snapshot standard the daemon applies to its viewer SSE
//!   subscribers).
//!
//! The writer's `select!` is BIASED, control lane first: any control
//! frame enqueued before a ring frame was broadcast reaches the wire
//! first. That is the ordering guarantee — identify/auth/snapshots
//! precede every delta ([`HostServer::attach_channel`] queues them in
//! the same `attach_lock` hold that subscribes the ring), and a
//! `LaboratoryCreated` precedes the first ring frame of a recreated
//! lab. This loop never writes a frame itself.

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use objectiveai_sdk::laboratories::daemon::{ChannelRequest, HostCommandResponse};
use tokio_tungstenite::tungstenite::Message;

use crate::host::HostServer;

/// Re-derive the `ws://`/`wss://` WebSocket scheme from the daemon's
/// published `http://`/`https://` address for the `/laboratory` dial
/// (the daemon's one WebSocket). An already-`ws://` address (legacy
/// config entry) passes through unchanged.
fn http_to_ws(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if let Some(rest) = url.strip_prefix("https://") {
        format!("wss://{rest}")
    } else {
        url.to_string()
    }
}

pub async fn run(
    daemon_address: String,
    signature: Option<String>,
    host: Arc<HostServer>,
    suppress_output: bool,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) {
    // The daemon publishes an `http://` address — its command channel,
    // broadcast, and SSE watcher routes are all plain HTTP. `/laboratory`
    // is its ONE WebSocket, so re-derive the `ws://` scheme here, at the
    // single site that actually dials it.
    let base = http_to_ws(daemon_address.trim_end_matches('/'));
    let url = format!("{base}/laboratory");
    // Mirrors the SDK `AuthEnvelope` shape without pulling the cli
    // feature in for one two-field struct.
    let auth_frame = serde_json::json!({ "signature": signature }).to_string();

    loop {
        // Cooperative cancel (a stdin `remove_address`): checked at
        // every await point so a removed address's task ends instead
        // of reconnecting — and, when connected, tears down through
        // the SAME detach path a natural disconnect takes, so no
        // channel registration ever leaks.
        let connect = tokio::select! {
            _ = cancel.changed() => return,
            connect = tokio_tungstenite::connect_async(&url) => connect,
        };
        match connect {
            Ok((ws, _)) => {
                if !suppress_output {
                    eprintln!("connected: {url}");
                }
                // TCP keepalive on the host↔daemon channel: a
                // silently-dead daemon must surface as a recv error so
                // the detach path runs instead of the host serving a
                // ghost forever.
                match ws.get_ref() {
                    tokio_tungstenite::MaybeTlsStream::Plain(tcp) => {
                        objectiveai_sdk::net::set_tcp_keepalive(tcp)
                    }
                    tokio_tungstenite::MaybeTlsStream::Rustls(tls) => {
                        objectiveai_sdk::net::set_tcp_keepalive(tls.get_ref().0)
                    }
                    _ => {}
                }
                let (mut sink, mut stream) = ws.split();
                let (reply_tx, mut reply_rx) =
                    tokio::sync::mpsc::unbounded_channel::<String>();
                // Attach FIRST: enqueues identify + auth + snapshots on
                // the control lane and subscribes the filetree ring,
                // atomically against broadcasts and folds. The frames
                // wait in the unbounded control queue until the writer
                // starts.
                let (channel_id, mut filetree_rx) =
                    host.attach_channel(reply_tx.clone(), auth_frame.clone()).await;
                // The single writer merges both lanes (see the module
                // docs for the lane contract and why `biased` control-
                // first is the ordering guarantee).
                let writer_host = Arc::clone(&host);
                let writer = tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            biased;
                            frame = reply_rx.recv() => match frame {
                                Some(frame) => {
                                    if sink.send(Message::Text(frame)).await.is_err() {
                                        break;
                                    }
                                }
                                // All control senders dropped — the
                                // channel is detached; we're done.
                                None => break,
                            },
                            result = filetree_rx.recv() => match result {
                                Ok(frame) => {
                                    if sink.send(Message::Text(frame)).await.is_err() {
                                        break;
                                    }
                                }
                                // Fell behind the ring: the missed
                                // events are already folded into the
                                // host's materialized trees — resync
                                // with fresh snapshots and resume.
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                    let mut failed = false;
                                    for frame in writer_host.filetree_snapshot_frames() {
                                        if sink.send(Message::Text(frame)).await.is_err() {
                                            failed = true;
                                            break;
                                        }
                                    }
                                    if failed {
                                        break;
                                    }
                                }
                                // Unreachable in practice: our
                                // `writer_host` Arc keeps the ring
                                // sender alive.
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                            },
                        }
                    }
                });
                let mut cancelled = false;
                loop {
                    let frame = tokio::select! {
                        _ = cancel.changed() => {
                            cancelled = true;
                            break;
                        }
                        frame = stream.next() => match frame {
                            Some(frame) => frame,
                            None => break,
                        },
                    };
                    let text = match frame {
                        Ok(Message::Text(text)) => text,
                        Ok(Message::Close(_)) | Err(_) => break,
                        // Pings are answered by tungstenite itself;
                        // ignore everything else.
                        Ok(_) => continue,
                    };
                    // ChannelRequest first (it requires `payload`,
                    // which a command frame never carries), then the
                    // daemon's multi-frame HostCommandResponse — the
                    // mirror of the daemon's own demux order.
                    let Ok(request) =
                        serde_json::from_str::<ChannelRequest>(&text)
                    else {
                        if let Ok(response) =
                            serde_json::from_str::<HostCommandResponse>(&text)
                        {
                            host.bridge().deliver(response);
                        }
                        // Forward-compat: skip frames this build
                        // doesn't know.
                        continue;
                    };
                    let host = Arc::clone(&host);
                    let reply_tx = reply_tx.clone();
                    tokio::spawn(async move {
                        let response = host.handle(channel_id, request).await;
                        if let Ok(frame) = serde_json::to_string(&response) {
                            let _ = reply_tx.send(frame);
                        }
                    });
                }
                host.detach_channel(channel_id);
                drop(reply_tx);
                let _ = writer.await;
                if cancelled {
                    // Removed from the dial list — done, no retry.
                    return;
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
        tokio::select! {
            _ = cancel.changed() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
        }
    }
}
