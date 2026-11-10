//! The frontend, beneath `/frontend`: the page, and one live socket per
//! open page, through which the person is the agent.
//!
//! The rules:
//!
//! 1. `GET /frontend` sends the page on to `/frontend/`, where it is
//!    served whole: one HTML file, its script and its style inline.
//! 2. `GET /frontend/ws` is the desk, live. On opening it sends who the
//!    page is (`me`), the transcript and whether the agent is waiting
//!    (`hello`), then the tools the agent may call (`tools`); after
//!    that, every chunk recorded (`entry`), every turn opened or ended
//!    (`state`), and trouble no act owns (`trouble`).
//! 3. A text frame from the page is one act, a JSON object by `type`:
//!    `reply` with `text`, `call` with `name` and `arguments` (an
//!    object), `end`, or `tools`, which lists them again. An act the
//!    desk refuses, or one that does not read, is answered on that
//!    socket alone with `refused` and why.
//! 4. A request with no stamp, or one stamped as an agent, is `403`.

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message as Frame, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use diverge_sdk::shared::Identity;
use rmcp::model::JsonObject;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::broadcast::error::RecvError;

use crate::caller::person;
use crate::desk::Desk;

/// The page, whole.
const PAGE: &str = include_str!("page.html");

/// The routes beneath `/frontend`, holding the desk.
pub fn routes(desk: Desk) -> Router {
    Router::new()
        .route("/frontend", get(|| async { Redirect::permanent("/frontend/") }))
        .route("/frontend/", get(page))
        .route("/frontend/ws", get(socket))
        .with_state(desk)
}

/// Rule 4.
fn refused(why: String) -> Response {
    (StatusCode::FORBIDDEN, axum::Json(json!({ "error": why }))).into_response()
}

/// Rule 1.
async fn page(headers: HeaderMap) -> Response {
    match person(&headers) {
        Ok(_) => Html(PAGE).into_response(),
        Err(why) => refused(why),
    }
}

/// Rule 2: the upgrade, answered once the stamp is read.
async fn socket(State(desk): State<Desk>, headers: HeaderMap, upgrade: WebSocketUpgrade) -> Response {
    match person(&headers) {
        Ok(identity) => upgrade.on_upgrade(move |socket| sit(socket, desk, identity)),
        Err(why) => refused(why),
    }
}

/// One act of the page's (rule 3).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Act {
    Reply { text: String },
    Call { name: String, arguments: JsonObject },
    End,
    Tools,
}

/// One message to the page.
async fn send(socket: &mut WebSocket, value: Value) -> bool {
    socket.send(Frame::Text(value.to_string().into())).await.is_ok()
}

/// Rules 2 and 3: the page at the desk, until it closes the socket or
/// the socket breaks.
async fn sit(mut socket: WebSocket, desk: Desk, identity: Identity) {
    // Listen before reading the transcript, so nothing recorded between
    // the two is lost; what the transcript already holds is skipped.
    let mut told = desk.listen();
    let (transcript, waiting) = desk.snapshot().await;
    let mut seen = transcript.last().map_or(-1, |entry| entry.id);
    if !send(&mut socket, json!({ "type": "me", "identity": identity })).await {
        return;
    }
    if !send(&mut socket, json!({ "type": "hello", "transcript": transcript, "waiting": waiting })).await {
        return;
    }
    if !send(&mut socket, tools(&desk).await).await {
        return;
    }
    loop {
        tokio::select! {
            heard = told.recv() => match heard {
                Ok(told) => {
                    let value = serde_json::to_value(&told).unwrap_or(Value::Null);
                    if let Some(id) = value.get("entry").and_then(|entry| entry.get("id")).and_then(Value::as_i64) {
                        if id <= seen {
                            continue;
                        }
                        seen = id;
                    }
                    if !send(&mut socket, value).await {
                        return;
                    }
                }
                // The page fell behind: it is told, and opens again.
                Err(RecvError::Lagged(_)) => {
                    let _ = send(&mut socket, json!({ "type": "lagged" })).await;
                    return;
                }
                Err(RecvError::Closed) => return,
            },
            frame = socket.recv() => match frame {
                Some(Ok(Frame::Text(text))) => {
                    let answer = match serde_json::from_str::<Act>(&text) {
                        Ok(Act::Tools) => Some(tools(&desk).await),
                        Ok(act) => act_on(&desk, act).await.err().map(|why| json!({ "type": "refused", "error": why })),
                        Err(error) => Some(json!({ "type": "refused", "error": format!("the act does not read: {error}") })),
                    };
                    if let Some(answer) = answer {
                        if !send(&mut socket, answer).await {
                            return;
                        }
                    }
                }
                Some(Ok(Frame::Close(_))) | None | Some(Err(_)) => return,
                Some(Ok(_)) => {}
            },
        }
    }
}

/// An act done at the desk. A tool call is placed here, where it may be
/// refused; the call itself runs on a task of its own, so the socket
/// goes on while it does, and its response is told to every page.
async fn act_on(desk: &Desk, act: Act) -> Result<(), String> {
    match act {
        Act::Reply { text } => {
            if text.trim().is_empty() {
                return Err("a reply needs text".to_string());
            }
            desk.reply(text).await
        }
        Act::Call { name, arguments } => {
            let placed = desk.place(name, arguments).await?;
            tokio::spawn(placed.finish());
            Ok(())
        }
        Act::End => desk.end().await,
        Act::Tools => Ok(()),
    }
}

/// The tools, as the page is told them.
async fn tools(desk: &Desk) -> Value {
    match desk.tools().await {
        Ok(tools) => json!({ "type": "tools", "tools": tools }),
        Err(error) => json!({ "type": "tools", "tools": [], "error": error }),
    }
}
