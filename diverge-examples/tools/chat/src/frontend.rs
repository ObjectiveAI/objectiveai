//! The frontend, beneath `/frontend`: the page, what it reads, and one
//! live socket per open tab.
//!
//! The rules:
//!
//! 1. `GET /frontend` sends the page on to `/frontend/`, where it is
//!    served whole: one HTML file, its script and its style inline.
//! 2. `GET /frontend/api/me` is the person the request came from, as the
//!    daemon stamped it on the `diverge.network` header: the account and
//!    the scope.
//! 3. `GET /frontend/api/agents` is every agent attached to the chat,
//!    with its image's name and `/DIVERGE.md`.
//! 4. `GET /frontend/ws?template=…&index=…` is the conversation with that
//!    agent, live: on opening, the whole history, oldest first; then
//!    every message written to it, the person's and the agent's, as it
//!    is kept. A text frame from the page is the person's message: it is
//!    kept under the stamped account, shown to every page open on the
//!    conversation, and the agent is told one is waiting.
//! 5. A request with no stamp, or one stamped as an agent, is `403`.

use axum::extract::ws::{Message as Frame, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use diverge_sdk::shared::Identity;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::broadcast::error::RecvError;

use crate::caller::{Participant, person};
use crate::chat::Chat;
use crate::failure::Failure;
use crate::store;

/// The page, whole.
const PAGE: &str = include_str!("page.html");

/// The routes beneath `/frontend`, holding the chat.
pub fn routes(chat: Chat) -> Router {
    Router::new()
        .route("/frontend", get(|| async { Redirect::permanent("/frontend/") }))
        .route("/frontend/", get(page))
        .route("/frontend/api/me", get(me))
        .route("/frontend/api/agents", get(agents))
        .route("/frontend/ws", get(socket))
        .with_state(chat)
}

/// A failure, as the page reads it.
fn refused(status: StatusCode, failure: Failure) -> Response {
    (status, Json(json!({ "error": failure.to_string() }))).into_response()
}

/// Rule 1.
async fn page(headers: HeaderMap) -> Response {
    match person(&headers) {
        Ok(_) => Html(PAGE).into_response(),
        Err(failure) => refused(StatusCode::FORBIDDEN, failure),
    }
}

/// Rule 2.
async fn me(headers: HeaderMap) -> Response {
    match person(&headers) {
        Ok((identity, scope)) => Json(json!({ "identity": identity, "scope": scope })).into_response(),
        Err(failure) => refused(StatusCode::FORBIDDEN, failure),
    }
}

/// Rule 3.
async fn agents(State(chat): State<Chat>, headers: HeaderMap) -> Response {
    if let Err(failure) = person(&headers) {
        return refused(StatusCode::FORBIDDEN, failure);
    }
    match chat.state.daemon.attached(&chat.state.client).await {
        Ok(agents) => Json(json!({ "agents": agents })).into_response(),
        Err(failure) => refused(StatusCode::BAD_GATEWAY, failure),
    }
}

/// Which conversation a socket is opened on.
#[derive(Debug, Deserialize)]
struct Which {
    template: String,
    index: i64,
}

/// Rule 4: the upgrade, answered once the stamp is read.
async fn socket(State(chat): State<Chat>, headers: HeaderMap, Query(which): Query<Which>, upgrade: WebSocketUpgrade) -> Response {
    let (identity, _scope) = match person(&headers) {
        Ok(person) => person,
        Err(failure) => return refused(StatusCode::FORBIDDEN, failure),
    };
    let agent = Participant { template: which.template, index: which.index, name: None };
    upgrade.on_upgrade(move |socket| converse(socket, chat, agent, identity))
}

/// Rule 4: the history, then the conversation live, until the page
/// closes the socket or the socket breaks.
async fn converse(mut socket: WebSocket, chat: Chat, agent: Participant, identity: Identity) {
    let state = &chat.state;
    // Listen before reading the history, so nothing written between the
    // two is lost; what the history already holds is skipped.
    let mut room = state.room(&agent.template, agent.index).await.subscribe();
    let pool = match state.store.pool(&state.client).await {
        Ok(pool) => pool,
        Err(failure) => {
            let _ = socket.send(Frame::Text(json!({ "type": "error", "error": failure.to_string() }).to_string().into())).await;
            return;
        }
    };
    let history = match store::history(pool, &agent.template, agent.index).await {
        Ok(history) => history,
        Err(failure) => {
            let _ = socket.send(Frame::Text(json!({ "type": "error", "error": failure.to_string() }).to_string().into())).await;
            return;
        }
    };
    let mut seen = history.last().map(|message| message.id).unwrap_or(0);
    if socket.send(Frame::Text(json!({ "type": "history", "messages": history }).to_string().into())).await.is_err() {
        return;
    }
    loop {
        tokio::select! {
            told = room.recv() => match told {
                Ok(message) => {
                    if message.id <= seen {
                        continue;
                    }
                    seen = message.id;
                    if socket.send(Frame::Text(json!({ "type": "message", "message": message }).to_string().into())).await.is_err() {
                        return;
                    }
                }
                // The page fell behind: it is told, and reads the
                // history again.
                Err(RecvError::Lagged(_)) => {
                    if socket.send(Frame::Text(json!({ "type": "lagged" }).to_string().into())).await.is_err() {
                        return;
                    }
                }
                Err(RecvError::Closed) => return,
            },
            frame = socket.recv() => match frame {
                Some(Ok(Frame::Text(text))) => {
                    let text = text.to_string();
                    if text.trim().is_empty() {
                        continue;
                    }
                    match store::user(pool, &agent, &identity.issuer, &identity.subject, &text).await {
                        Ok(message) => {
                            state.told(&message).await;
                            if let Err(failure) = state.daemon.notify(&state.client, &agent).await {
                                let _ = socket.send(Frame::Text(json!({ "type": "error", "error": failure.to_string() }).to_string().into())).await;
                            }
                        }
                        Err(failure) => {
                            let _ = socket.send(Frame::Text(json!({ "type": "error", "error": failure.to_string() }).to_string().into())).await;
                        }
                    }
                }
                Some(Ok(Frame::Close(_))) | None | Some(Err(_)) => return,
                Some(Ok(_)) => {}
            },
        }
    }
}
