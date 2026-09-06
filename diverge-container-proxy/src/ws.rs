//! The WebSocket half: `/requests` in, and every answer path.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt as _, StreamExt as _};

use crate::requests::{Answering, Claim, Kind, Refusal, Requests};

/// Accept the one `/requests` connection, or refuse because one is
/// live.
///
/// The claim is taken BEFORE the upgrade, so two arrivals cannot both
/// pass the door; the loser is told `409` and gets no socket. The
/// claim rides into the upgrade callback and is dropped on every exit
/// from it — and if the upgrade never completes, dropped with the
/// callback, which is what keeps a failed arrival from wedging the
/// slot shut.
pub async fn requests(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let Some(claim) = requests.claim().await else {
        return StatusCode::CONFLICT.into_response();
    };
    upgrade
        .on_upgrade(move |socket| serve_requests(socket, requests, claim))
        .into_response()
}

/// Serve the `/requests` connection until it ends, then release.
///
/// A write pump owns the sink: askers queue encoded frames and never
/// touch the socket, so a slow server slows the queue and nothing
/// else. The server sends NOTHING on this path, so the read loop
/// exists to notice the connection ending — and a message from the
/// server is a peer speaking something else, which ends it too.
async fn serve_requests(socket: WebSocket, requests: Arc<Requests>, claim: Claim) {
    let (mut sink, mut stream) = socket.split();

    let (sender, mut queue) = Requests::queue();
    let mut pump = tokio::spawn(async move {
        while let Some(frame) = queue.recv().await {
            if sink.send(Message::Binary(frame.into())).await.is_err() {
                break;
            }
        }
    });

    requests.publish(&claim, sender).await;

    while let Some(Ok(message)) = stream.next().await {
        match message {
            Message::Binary(_) | Message::Text(_) | Message::Close(_) => break,
            // Pings are answered by axum; pongs carry nothing.
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }

    // The claim's drop releases the slot and kills the asks no answer
    // path has opened; aborting the pump drops the sink, which closes
    // the socket.
    drop(claim);
    pump.abort();
    let _ = (&mut pump).await;
}

/// Accept an answer path for one channel, or refuse: `404` for a
/// channel that is not an ask of this kind awaiting its answer, `409`
/// for one already being answered — both BEFORE the upgrade.
async fn answer(
    kind: Kind,
    requests: Arc<Requests>,
    upgrade: WebSocketUpgrade,
    channel: u32,
) -> Response {
    let answering = match requests.open(kind, channel).await {
        Ok(answering) => answering,
        Err(Refusal::NotFound) => return StatusCode::NOT_FOUND.into_response(),
        Err(Refusal::Conflict) => return StatusCode::CONFLICT.into_response(),
    };
    upgrade
        .on_upgrade(move |socket| serve_answer(socket, requests, answering))
        .into_response()
}

/// Read one answer until its socket ends.
///
/// Every binary message is one message of the answer, delivered as
/// it arrives. A `Close` is the answer whole; the stream ending any
/// other way — dropped, errored, or a text message from a peer
/// speaking something else — is the answer dead. The proxy never
/// writes on an answer path.
async fn serve_answer(
    mut socket: WebSocket,
    requests: Arc<Requests>,
    answering: Answering,
) {
    let mut complete = false;
    while let Some(Ok(message)) = socket.next().await {
        match message {
            Message::Binary(bytes) => requests.deliver(&answering, bytes),
            Message::Close(_) => {
                complete = true;
                break;
            }
            Message::Text(_) => break,
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }
    requests.finish(answering, complete).await;
}

/// `/mcp/list-tools/{channel}`.
pub async fn mcp_list_tools(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::McpListTools, requests, upgrade, channel).await
}

/// `/mcp/list-resources/{channel}`.
pub async fn mcp_list_resources(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::McpListResources, requests, upgrade, channel).await
}

/// `/mcp/call-tool/{channel}`.
pub async fn mcp_call_tool(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::McpCallTool, requests, upgrade, channel).await
}

/// `/mcp/read-resource/{channel}`.
pub async fn mcp_read_resource(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::McpReadResource, requests, upgrade, channel).await
}

/// `/mcp/notifications/{channel}`.
pub async fn mcp_notifications(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::McpNotifications, requests, upgrade, channel).await
}

/// `/vault/get/{channel}`.
pub async fn vault_get(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::VaultGet, requests, upgrade, channel).await
}

/// `/vault/set/{channel}`.
pub async fn vault_set(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::VaultSet, requests, upgrade, channel).await
}

/// `/vault/delete/{channel}`.
pub async fn vault_delete(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::VaultDelete, requests, upgrade, channel).await
}

/// `/vault/lock/{channel}`.
pub async fn vault_lock(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::VaultLock, requests, upgrade, channel).await
}

/// `/vault/unlock/{channel}`.
pub async fn vault_unlock(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::VaultUnlock, requests, upgrade, channel).await
}

/// `/command/{channel}`.
pub async fn command(
    State(requests): State<Arc<Requests>>,
    upgrade: WebSocketUpgrade,
    Path(channel): Path<u32>,
) -> Response {
    answer(Kind::Command, requests, upgrade, channel).await
}
