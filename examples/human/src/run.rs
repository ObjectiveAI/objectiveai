//! `POST /run`, `POST /enqueue` and `POST /dequeue`: the loop, which
//! is the person.
//!
//! The rules:
//!
//! 1. A run before the registration is `409`, `unregistered`; a run
//!    with no message, or a message with no content, is `400`; a run
//!    while a turn is open is `409`, `busy`.
//! 2. Otherwise the run is `200`, server-sent events: the messages'
//!    parts first, then whatever the person does, each chunk one
//!    event's data, until the person ends the turn, which ends the
//!    stream. Nothing times it out: the agent waits on the person.
//! 3. A comment goes out on the stream when it is quiet, so the
//!    connection is never idle for long; the proxy reads only data.
//! 4. An enqueue with no message, or a message with no content, is
//!    `400`; otherwise its messages — one message, as the proxy joined
//!    them — are the desk's to take, `delivered` while a turn is open
//!    and `missed` with none.
//! 5. A dequeue is always `empty`: a message is taken the moment it
//!    comes, so none is ever held to withdraw.

use std::convert::Infallible;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use diverge_sdk::container_proxy::inside::agent::dequeue::{self, Outcome};
use diverge_sdk::container_proxy::inside::agent::enqueue::{self, Fate};
use diverge_sdk::container_proxy::inside::agent::run;
use futures_util::Stream;
use serde_json::{Value, json};

use crate::desk::{Busy, Desk};
use crate::register::registered;

/// A refusal: the status, and a JSON reason.
type Refusal = (StatusCode, Json<Value>);

/// Rules 1 to 3.
pub async fn run(State(desk): State<Desk>, Json(request): Json<run::request::Request>) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Refusal> {
    if !registered() {
        return Err((StatusCode::CONFLICT, Json(json!({ "kind": "unregistered", "error": "the agent is not registered" }))));
    }
    if request.messages.is_empty() || request.messages.iter().any(|message| message.content.is_empty()) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "kind": "content", "error": "a run needs messages, each with content" }))));
    }
    let messages = request.messages.into_iter().map(|message| (message.key, message.content)).collect();
    let mut chunks = match desk.begin(messages).await {
        Ok(chunks) => chunks,
        Err(Busy::Open) => return Err((StatusCode::CONFLICT, Json(json!({ "kind": "busy", "error": "a turn is open" })))),
    };
    let events = async_stream::stream! {
        while let Some(chunk) = chunks.recv().await {
            match serde_json::to_string(&chunk) {
                Ok(data) => yield Ok(Event::default().data(data)),
                Err(_) => continue,
            }
        }
    };
    Ok(Sse::new(events).keep_alive(KeepAlive::default()))
}

/// Rule 4.
pub async fn enqueue(State(desk): State<Desk>, Json(request): Json<enqueue::request::Request>) -> Result<Json<Fate>, Refusal> {
    if request.messages.is_empty() || request.messages.iter().any(|message| message.content.is_empty()) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "kind": "content", "error": "an enqueue needs messages, each with content" }))));
    }
    let messages = request.messages.into_iter().map(|message| (message.key, message.content)).collect();
    Ok(Json(desk.deliver(messages).await))
}

/// Rule 5.
pub async fn dequeue(Json(_request): Json<dequeue::request::Request>) -> Json<Outcome> {
    Json(Outcome::Empty)
}
