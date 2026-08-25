//! The WebSocket half: one connection at `/`, serving the wire.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::mcp_proxy;
use futures_util::{SinkExt as _, StreamExt as _};

use crate::proxy::{Claim, Proxy};

/// Accept the one connection, or refuse because one is live.
///
/// The claim is taken BEFORE the upgrade, so two arrivals cannot both
/// pass the door; the loser is told `409` and gets no socket. The
/// claim rides into the upgrade callback and is dropped on every exit
/// from it — and if the upgrade never completes, dropped with the
/// callback, which is what keeps a failed arrival from wedging the
/// slot shut.
pub async fn accept(
    State(proxy): State<Arc<Proxy>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let Some(claim) = proxy.claim().await else {
        return StatusCode::CONFLICT.into_response();
    };
    upgrade
        .on_upgrade(move |socket| serve(socket, proxy, claim))
        .into_response()
}

/// Serve one connection until it ends, then release.
///
/// A write pump owns the sink: handlers queue encoded frames and never
/// touch the socket, so a slow peer slows the queue and nothing else.
/// The read loop routes the server's answers. Either side stopping —
/// a close, an error, a frame that would not decode — ends both, and
/// the claim's drop kills whatever channels were riding the
/// connection.
async fn serve(socket: WebSocket, proxy: Arc<Proxy>, claim: Claim) {
    let (mut sink, mut stream) = socket.split();

    let (sender, mut queue) = Proxy::queue();
    let mut pump = tokio::spawn(async move {
        while let Some(frame) = queue.recv().await {
            if sink.send(Message::Binary(frame.into())).await.is_err() {
                break;
            }
        }
    });

    proxy.publish(&claim, sender).await;

    while let Some(Ok(message)) = stream.next().await {
        match message {
            Message::Binary(bytes) => {
                match mcp_proxy::server::Frame::decode(&bytes) {
                    Ok(mcp_proxy::server::Frame::ChannelResponse {
                        channel,
                        payload,
                    }) => {
                        proxy.respond(channel, bytes.slice_ref(payload)).await
                    }
                    Ok(mcp_proxy::server::Frame::ChannelResponseFinish {
                        channel,
                    }) => proxy.finish(channel).await,
                    // A peer speaking something else. There is no
                    // answering a frame that could not be read, and no
                    // reading past it either — the stream has lost its
                    // framing.
                    Err(_) => break,
                }
            }
            // Every message on this wire is BINARY; text is a peer
            // speaking something else.
            Message::Text(_) => break,
            Message::Close(_) => break,
            // Pings are answered by axum; pongs carry nothing.
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }

    // The claim's drop releases the slot and kills the live channels;
    // aborting the pump drops the sink, which closes the socket.
    drop(claim);
    pump.abort();
    let _ = (&mut pump).await;
}
