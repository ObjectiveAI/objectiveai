//! `/agent/enqueue`: a message's fate, forwarded from `POST /enqueue`.

use std::pin::pin;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::container_proxy::agent::enqueue::Fate;
use diverge_provider_sdk::shared::containers::enqueue::response;
use diverge_provider_sdk::shared::error::Error;
use futures_util::future;
use futures_util::StreamExt as _;
use reqwest::header::CONTENT_TYPE;

use super::{Upstream, upstream};
use crate::ws;

/// `/agent/enqueue`. Accepted as many times as the server opens it,
/// each one message.
pub async fn enqueue(State(upstream): State<Arc<Upstream>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, upstream))
        .into_response()
}

/// Serve one enqueue.
///
/// The first message is the request, forwarded as the `POST /enqueue`
/// body verbatim; anything else is the clean close with nothing
/// before it. The call is then held for as long as the agent's server
/// holds it — the fate comes when it is known, and nothing times it
/// out — with the server's socket watched meanwhile: a server that
/// leaves drops the call. A `2xx` is the fate, as the frame; a
/// non-`2xx`, a server that cannot be dialed, or a body that is not
/// a fate is `Error`. Then the close.
async fn serve(socket: WebSocket, upstream: Arc<Upstream>) {
    let (sink, mut stream) = socket.split();
    let Some(request) = ws::binary(&mut stream).await else {
        upstream::finish(sink, None).await;
        return;
    };

    let mut sending = pin!(
        upstream
            .http()
            .post(upstream.url("/enqueue"))
            .header(CONTENT_TYPE, "application/json")
            .body(request)
            .send()
    );
    let response = loop {
        let reading = pin!(stream.next());
        match future::select(sending.as_mut(), reading).await {
            future::Either::Left((response, _)) => break response,
            future::Either::Right((Some(Ok(Message::Text(_) | Message::Ping(_) | Message::Pong(_))), _)) => {}
            future::Either::Right(_) => return,
        }
    };

    let frame = match response {
        Err(error) => response::Frame::Error(upstream::refused(&error)),
        Ok(response) if !response.status().is_success() => {
            response::Frame::Error(upstream::status_error(response).await)
        }
        Ok(response) => match response.json::<Fate>().await {
            Ok(fate) => response::Frame::from(fate),
            Err(error) => response::Frame::Error(Error(serde_json::json!({
                "kind": "agent",
                "error": format!("the fate did not parse: {error}"),
            }))),
        },
    };
    upstream::finish(sink, upstream::encoded(&frame)).await;
}
