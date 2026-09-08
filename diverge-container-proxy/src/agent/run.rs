//! `/agent/run`: the loop, forwarded from the agent's `POST /run`.

use std::pin::pin;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::shared::containers::run_loop::response;
use diverge_provider_sdk::shared::error::Error;
use eventsource_stream::Eventsource as _;
use futures_util::future;
use futures_util::{SinkExt as _, StreamExt as _};
use reqwest::header::CONTENT_TYPE;

use super::{Upstream, upstream};

/// `/agent/run`. Accepted as many times as the server opens it; one
/// loop at a time is the agent's server's rule, and its refusal is
/// forwarded like any other.
pub async fn run(State(upstream): State<Arc<Upstream>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, upstream))
        .into_response()
}

/// Serve one run.
///
/// The rules, in order:
///
/// 1. The first message is the request. It must be binary; anything
///    else is the clean close with nothing before it — the server
///    sent something this is not. Its bytes are not read here: they
///    are the `POST /run` body, verbatim.
/// 2. A server that cannot be dialed is `Error`, at once, then the
///    close. A non-`2xx` is `Error` with its body, then the close.
///    Either way the loop never ran.
/// 3. A `2xx` is the loop: every event's data goes out as one
///    `Chunk`, the JSON untouched behind the tag. The stream's end is
///    the clean close. The stream dying — the connection ending
///    without its end — is `Error`, then the close, because a stream
///    that simply stopped could not be told from one that finished.
/// 4. The server is watched throughout: a server that leaves drops
///    the response, which ends the call to the agent's server.
async fn serve(socket: WebSocket, upstream: Arc<Upstream>) {
    let (sink, mut stream) = socket.split();
    let request = match stream.next().await {
        Some(Ok(Message::Binary(bytes))) => bytes,
        _ => {
            upstream::finish(sink, None).await;
            return;
        }
    };

    let response = match upstream
        .http()
        .post(upstream.url("/run"))
        .header(CONTENT_TYPE, "application/json")
        .body(request)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let frame = response::Frame::Error(upstream::refused(&error));
            upstream::finish(sink, upstream::encoded(&frame)).await;
            return;
        }
    };
    if !response.status().is_success() {
        let frame = response::Frame::Error(upstream::status_error(response).await);
        upstream::finish(sink, upstream::encoded(&frame)).await;
        return;
    }

    let mut sink = sink;
    let mut events = pin!(response.bytes_stream().eventsource());
    loop {
        let next = pin!(events.next());
        let reading = pin!(stream.next());
        match future::select(next, reading).await {
            future::Either::Left((Some(Ok(event)), _)) => {
                // The chunk's tag, then its JSON as the agent's server
                // wrote it.
                let mut bytes = Vec::with_capacity(1 + event.data.len());
                bytes.push(response::CHUNK);
                bytes.extend_from_slice(event.data.as_bytes());
                if sink.send(Message::Binary(bytes.into())).await.is_err() {
                    return;
                }
            }
            future::Either::Left((Some(Err(_)), _)) => {
                let frame = response::Frame::Error(Error(serde_json::json!({
                    "kind": "loop",
                    "error": "the loop ended without finishing",
                })));
                upstream::finish(sink, upstream::encoded(&frame)).await;
                return;
            }
            future::Either::Left((None, _)) => {
                upstream::finish(sink, None).await;
                return;
            }
            future::Either::Right((Some(Ok(Message::Ping(_) | Message::Pong(_))), _)) => {}
            future::Either::Right(_) => return,
        }
    }
}
