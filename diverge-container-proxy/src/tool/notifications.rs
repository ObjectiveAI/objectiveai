//! `/tool/notifications`: what the container's server says on its own
//! account, for as long as the path is open.

use std::pin::pin;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::shared::mcp::notifications::response;
use futures_util::{SinkExt as _, StreamExt as _, future};
use tokio::sync::broadcast;

use super::Tool;
use crate::agent;

/// `/tool/notifications`. Accepted as many times as the server opens
/// it; each opening hears everything from then on.
pub async fn notifications(State(tool): State<Arc<Tool>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, tool))
        .into_response()
}

/// Subscribe, then relay: one frame per notification until the server
/// leaves. The client is dialled first so a container whose server
/// cannot be reached answers `Error` at once rather than a silence
/// that could mean anything; a subscriber that fell behind loses the
/// oldest and goes on, which the stream does not report — a
/// notification is a nudge, not a record.
async fn serve(socket: WebSocket, tool: Arc<Tool>) {
    let (mut sink, mut stream) = socket.split();
    let mut receiver = tool.subscribe();
    if let Err(error) = tool.client().await {
        agent::finish(sink, agent::encoded(&response::Frame::Error(error))).await;
        return;
    }
    loop {
        let next = pin!(receiver.recv());
        let reading = pin!(stream.next());
        match future::select(next, reading).await {
            future::Either::Left((Ok(notification), _)) => {
                let frame = response::Frame::Notification(notification);
                let Some(bytes) = agent::encoded(&frame) else {
                    continue;
                };
                if sink.send(Message::Binary(bytes.into())).await.is_err() {
                    return;
                }
            }
            future::Either::Left((Err(broadcast::error::RecvError::Lagged(_)), _)) => {}
            // The sender lives as long as the proxy: unreachable, and
            // the clean close is the honest answer if it ever were.
            future::Either::Left((Err(broadcast::error::RecvError::Closed), _)) => {
                agent::finish(sink, None).await;
                return;
            }
            future::Either::Right((Some(Ok(Message::Ping(_) | Message::Pong(_))), _)) => {}
            future::Either::Right(_) => return,
        }
    }
}
