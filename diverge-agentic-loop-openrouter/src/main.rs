//! The `openrouter` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `openrouter`, per the Container section of the
//! provider specification: a WebSocket at `/` on port 14978, the
//! server's first message the caller's request, every message back
//! one frame of the container response vocabulary — the turns behind
//! it run by [`r#loop`](r#loop::r#loop). The agent's tool calls go
//! out as an MCP client against the in-container proxy on port 14979.
//! Beside the run, the queue's two verbs: `POST /enqueue` and `POST
//! /dequeue`, per the SDK's `agentic_loop_container` module — the
//! caller's way into the conversation already running. The
//! continuation the run resumes from arrives on the `/continuation`
//! routes, into [`continuation_fetcher`]'s slot; `POST
//! /resource/{identity}` is served because the surface has it, and
//! answers honestly: an `openrouter` agent names no resources, so
//! every delivery is unrequested.

mod continuation;
mod continuation_fetcher;
mod fetch;
mod r#loop;
mod queue;
// The wire modules carry OpenRouter's COMPLETE shapes, which is more
// than this container constructs or reads — a role never built, a
// field parsed and never consumed. The derives used to count as use;
// now that each module keeps only its own direction's derive, the
// completeness reads as dead code, and is not.
#[allow(dead_code)]
mod request;
#[allow(dead_code)]
mod response;
mod serde_util;
mod stream_once;

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use diverge_provider_sdk::agentic_loop_container;
use diverge_provider_sdk::agentic_loop_container::response::Response;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use diverge_provider_sdk::CHUNK_SIZE;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::Agent;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::{
    AgenticLoopChunk, NotificationChunk,
};
use futures_util::StreamExt as _;

use crate::continuation::Continuation;
use crate::continuation_fetcher::FetchError;
use crate::queue::QUEUE;
use crate::r#loop::Item;

/// The loop port of the Container section of the provider
/// specification: where the server opens the run's socket.
const PORT: u16 = 14978;

/// Whether the container's one run has arrived.
///
/// A container is one run: its queue, its MCP session, its
/// filesystem are all one conversation's, and a second run would
/// share all of them with the first. So the FIRST socket claims the
/// container for good — an atomic swap, so two arrivals a nanosecond
/// apart resolve to exactly one winner — and everything after it,
/// concurrent or later, is refused with `409` before the upgrade: a
/// first run that fails every later check has still spent the
/// container, because "one run" is a fact about arrivals, not about
/// merit.
static CLAIMED: AtomicBool = AtomicBool::new(false);

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(run());
}

async fn run() {
    let app = axum::Router::new()
        .route("/", axum::routing::get(serve))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue))
        .route("/continuation", axum::routing::post(continuation_chunk))
        .route(
            "/continuation/complete",
            axum::routing::post(continuation_complete),
        )
        .route(
            "/continuation/error",
            axum::routing::post(continuation_error),
        )
        .route("/resource/{identity}", axum::routing::post(resource))
        .route(
            "/resource/{identity}/complete",
            axum::routing::post(resource_complete),
        )
        .route(
            "/resource/{identity}/error",
            axum::routing::post(resource_error),
        );

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// One socket, one run.
///
/// The upgrade is refused for the one thing knowable before the
/// request is read: a run after the first is the CALLER's error —
/// `409`, the container is [`CLAIMED`]. Everything after the upgrade
/// is the stream's: see [`drive`]. The socket closes when the drive
/// returns, whatever it returned for.
async fn serve(
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)>
{
    if CLAIMED.swap(true, Ordering::SeqCst) {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "claimed",
                "error": "this container serves one run, and it has already begun",
            })),
        ));
    }

    Ok(ws.on_upgrade(|mut socket| async move {
        drive(&mut socket).await;
        // The close is a frame of its own; the socket is gone when
        // the future is.
        let _ = socket.send(Message::Close(None)).await;
    }))
}

/// The run, on the socket: the request read, the continuation asked
/// for and awaited, the loop started, its items relayed — chunks as
/// chunk frames, the closer as continuation frames.
///
/// Every failure past the upgrade has no status left to set and says
/// so as a fatal `notification` — the loop's own vocabulary — after
/// which the socket closes:
///
/// - the first message not being the request frame, or an agent of
///   another kind: the CALLER's;
/// - a missing `OPENROUTER_API_KEY`: the server's own
///   misconfiguration — the caller's request carries no credential
///   for the upstream; whose key the container runs with is the
///   image's business;
/// - a continuation the server could not deliver — its own words,
///   verbatim — or one that will not open;
/// - the loop failing to start, in OpenRouter's own words where it
///   has them.
///
/// Before the loop exists, each of those also CLOSES the queue: the
/// container is spent and no run is coming, so pending or future
/// enqueues are missed honestly instead of waiting forever. Once the
/// loop exists, its own guard does that however the run ends.
///
/// An error the loop reports mid-stream arrives as a fatal
/// notification and is the loop's last word — except that a loop
/// which died with progress worth keeping follows it with the
/// closer, the salvaged history.
async fn drive(socket: &mut WebSocket) {
    // The request: the first binary message, the wire's own frame.
    let request = match socket.recv().await {
        Some(Ok(Message::Binary(bytes))) => {
            match agentic_loop_container::request::Request::decode(&bytes) {
                Ok(request) => request,
                Err(error) => {
                    QUEUE.close().await;
                    fail(
                        socket,
                        serde_json::json!({
                            "kind": "request",
                            "error": error.to_string(),
                        }),
                    )
                    .await;
                    return;
                }
            }
        }
        _ => {
            QUEUE.close().await;
            fail(
                socket,
                serde_json::json!({
                    "kind": "request",
                    "error": "the first message was not the request frame",
                }),
            )
            .await;
            return;
        }
    };

    let Agent::Openrouter(agent) = request.agent else {
        QUEUE.close().await;
        fail(
            socket,
            serde_json::json!({
                "kind": "wrong_agent",
                "error": "this container serves the openrouter agent kind",
            }),
        )
        .await;
        return;
    };

    let api_key = match std::env::var("OPENROUTER_API_KEY") {
        Ok(api_key) => api_key,
        Err(_) => {
            QUEUE.close().await;
            fail(
                socket,
                serde_json::json!({
                    "kind": "missing_api_key",
                    "error": "OPENROUTER_API_KEY is not set",
                }),
            )
            .await;
            return;
        }
    };

    // The ask, then the wait: the server fetches the continuation
    // from the caller and delivers it on the routes.
    if !send(socket, &Response::FetchContinuation).await {
        QUEUE.close().await;
        return;
    }
    let continuation = match continuation_fetcher::STORE.fetch().await {
        Ok(None) => None,
        Ok(Some(chunks)) => match Continuation::parse(&chunks) {
            Ok(continuation) => Some(continuation),
            Err(error) => {
                QUEUE.close().await;
                fail(
                    socket,
                    serde_json::json!({
                        "kind": "continuation",
                        "error": error.to_string(),
                    }),
                )
                .await;
                return;
            }
        },
        Err(FetchError::Failed(error)) => {
            QUEUE.close().await;
            fail(
                socket,
                serde_json::json!({
                    "kind": "continuation",
                    "error": error,
                }),
            )
            .await;
            return;
        }
        Err(error) => {
            QUEUE.close().await;
            fail(
                socket,
                serde_json::json!({
                    "kind": "continuation",
                    "error": error.to_string(),
                }),
            )
            .await;
            return;
        }
    };

    let mut items =
        match r#loop::r#loop(&api_key, agent, continuation, request.prompt)
            .await
        {
            Ok(items) => items,
            Err(error) => {
                QUEUE.close().await;
                fail(socket, error.message()).await;
                return;
            }
        };

    while let Some(item) = items.next().await {
        let sent = match item {
            Ok(Item::Chunk(chunk)) => {
                send(socket, &Response::Chunk(chunk)).await
            }
            Ok(Item::Continuation(bytes)) => closer(socket, &bytes).await,
            // The stream has already begun; there is no status left
            // to change. The failure travels IN the stream, fatally,
            // and the loop ends right after it — with the closer,
            // when it died with progress worth keeping.
            Err(error) => {
                send(socket, &Response::Chunk(notification(error.message(), true)))
                    .await
            }
        };
        if !sent {
            return;
        }
    }
}

/// Send the closer: the continuation's bytes as continuation frames,
/// split at [`CHUNK_SIZE`]. Whether the socket stayed up.
async fn closer(socket: &mut WebSocket, bytes: &[u8]) -> bool {
    for piece in bytes.chunks(CHUNK_SIZE) {
        if !send(socket, &Response::Continuation(piece)).await {
            return false;
        }
    }
    true
}

/// A notification chunk, its fatality the caller's verdict.
fn notification(
    message: serde_json::Value,
    is_fatal: bool,
) -> AgenticLoopChunk {
    AgenticLoopChunk::Notification(NotificationChunk {
        r#type: Default::default(),
        is_fatal,
        message,
        meta: None,
    })
}

/// Write one frame as one binary message. Whether it went — a frame
/// that will not encode, or a socket that is gone, both end the run,
/// and nothing here can tell those apart or needs to.
async fn send(socket: &mut WebSocket, frame: &Response<'_>) -> bool {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_err() {
        return false;
    }
    socket.send(Message::Binary(buffer.into())).await.is_ok()
}

/// The run cannot go on: say so as the fatal notification. The
/// socket closes when the drive returns.
async fn fail(socket: &mut WebSocket, message: serde_json::Value) {
    let _ = send(socket, &Response::Chunk(notification(message, true))).await;
}

/// A message for the running conversation's queue.
///
/// The response IS the fate, and it arrives when the fate is known —
/// taken into the conversation, withdrawn by a dequeue, or outlived
/// by the run. That can be long after the ask; nothing here times
/// anything out. The one failure with no fate to report — the fate
/// channel dying, which the loop's close guard exists to prevent —
/// answers as HTTP does, with a status.
async fn enqueue(
    Json(request): Json<agentic_loop_container::enqueue::Request>,
) -> Result<
    Json<agentic_loop_container::enqueue::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    let fate = QUEUE.enqueue(request.prompt).await;
    match fate.await {
        // The queue speaks the SDK's own response type, so the fate
        // forwards as itself.
        Ok(response) => Ok(Json(response)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "kind": "fate_lost",
                "error": "the message's fate was never decided",
            })),
        )),
    }
}

/// One chunk of the continuation into the slot the run will read.
///
/// The body is the bytes verbatim, so nothing can be malformed — the
/// one refusal is a delivery after the settlement (`409`), because
/// anything after a settlement is somebody's bug.
async fn continuation_chunk(
    body: axum::body::Bytes,
) -> Result<
    Json<agentic_loop_container::continuation::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(continuation_fetcher::STORE.chunk(&body).await)
}

/// The completion: every chunk is in — or none at all, the fresh
/// start — and the run may collect.
async fn continuation_complete(
    Json(_request): Json<agentic_loop_container::continuation::complete::Request>,
) -> Result<
    Json<agentic_loop_container::continuation::complete::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(continuation_fetcher::STORE.complete().await)
}

/// The failure: the bytes can never come, and the waiting run learns
/// so in the server's own words.
async fn continuation_error(
    Json(request): Json<agentic_loop_container::continuation::error::Request>,
) -> Result<
    Json<agentic_loop_container::continuation::error::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(continuation_fetcher::STORE.error(request.error).await)
}

/// The continuation routes' one verdict: taken (`received`), or
/// refused because the delivery was already settled (`409`).
fn settled(
    taken: bool,
) -> Result<
    Json<agentic_loop_container::continuation::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    if taken {
        Ok(Json(agentic_loop_container::continuation::Response {
            r#type: Default::default(),
        }))
    } else {
        Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "settled",
                "error": "the continuation is already settled",
            })),
        ))
    }
}

/// A resource chunk, for a container that never asks for one.
///
/// The routes are the surface's, so they are served; the answer is
/// honest. An `openrouter` agent names no resources, so no ask ever
/// rides this container's socket and no delivery can be answering
/// one: `409`, on all three routes alike (a chunk cannot be
/// malformed — any bytes are one; the endings' JSON is judged by
/// the extractor).
async fn resource(
    axum::extract::Path(_identity): axum::extract::Path<String>,
    _body: axum::body::Bytes,
) -> Result<
    Json<agentic_loop_container::resource::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    Err(unrequested())
}

/// A resource completion — unrequested, as [`resource`] says.
async fn resource_complete(
    axum::extract::Path(_identity): axum::extract::Path<String>,
    Json(_request): Json<agentic_loop_container::resource::complete::Request>,
) -> Result<
    Json<agentic_loop_container::resource::complete::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    Err(unrequested())
}

/// A resource failure — unrequested, as [`resource`] says.
async fn resource_error(
    axum::extract::Path(_identity): axum::extract::Path<String>,
    Json(_request): Json<agentic_loop_container::resource::error::Request>,
) -> Result<
    Json<agentic_loop_container::resource::error::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    Err(unrequested())
}

/// The resource routes' one honest answer here.
fn unrequested() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "kind": "unrequested",
            "error": "this container never asks for resources",
        })),
    )
}

/// Clear the running conversation's queue.
///
/// Naive, deliberately: whatever is pending is withdrawn — each
/// message's own `/enqueue` answers `dequeued` — and a queue with
/// nothing pending, closed or not, answers `empty`.
async fn dequeue(
    Json(_request): Json<agentic_loop_container::dequeue::Request>,
) -> Json<agentic_loop_container::dequeue::Response> {
    if QUEUE.dequeue().await {
        Json(agentic_loop_container::dequeue::Response::Dequeued {
            r#type: Default::default(),
        })
    } else {
        Json(agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        })
    }
}
