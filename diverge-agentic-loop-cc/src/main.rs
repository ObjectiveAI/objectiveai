//! The `claude_code` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `claude_code`, per the Container section of the
//! provider specification: a WebSocket at `/` on port 14978, the
//! server's first message the caller's request, every message back
//! one frame of the container response vocabulary — the run itself a
//! Claude Code subprocess behind [`spawn`]. MCP is asked on port
//! 14979, Postgres opened to port 14980. Beside the run, the queue's
//! two verbs: `POST /enqueue` and `POST /dequeue`, per the SDK's
//! `agentic_loop_container` module — the caller's way into the
//! conversation already running: Claude Code holds the queue, and
//! this container holds the writer. The continuation the run resumes
//! from arrives on the `/continuation` routes, into
//! [`continuation_fetcher`]'s slot; `/resource/{identity}` is served
//! because the surface has it, and answers honestly: a `claude_code`
//! agent names no resources, so every delivery is unrequested.

mod continuation;
mod continuation_fetcher;
// The wire module carries Claude Code's COMPLETE stdout vocabulary,
// which is more than the conversion consumes — a field parsed and
// never read is the completeness, not dead code.
#[allow(dead_code)]
mod response;
mod spawn;

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use diverge_provider_sdk::agentic_loop_container;
use diverge_provider_sdk::agentic_loop_container::response::Response;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use diverge_provider_sdk::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::Agent;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::{
    AgenticLoopChunk, NotificationChunk,
};
use futures_util::StreamExt as _;

use crate::continuation::Continuation;
use crate::continuation_fetcher::FetchError;

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
    // The install begins the moment the process does — requests or
    // none. Endpoints await the same memoized outcome.
    tokio::spawn(async {
        let _ = spawn::installed().await;
    });

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
/// The upgrade is refused for what is knowable before the request is
/// read — and only that:
///
/// - A run after the first is the CALLER's error, and the first
///   error checked: `409`, the container is [`CLAIMED`].
/// - Claude Code failing to INSTALL — the harness fetches it at
///   startup; see [`spawn::installed`] — is the server's own,
///   checked right after the claim (arrival still spends the
///   container): `500`, and the same on every other endpoint,
///   forever. A request during the install simply waits for the
///   outcome.
///
/// Everything after the upgrade is the stream's: see [`drive`]. The
/// socket closes when the drive returns, whatever it returned for.
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

    installed().await?;

    Ok(ws.on_upgrade(|mut socket| async move {
        drive(&mut socket).await;
        // The close is a frame of its own; the socket is gone when
        // the future is.
        let _ = socket.send(Message::Close(None)).await;
    }))
}

/// The run, on the socket: the request read, the continuation asked
/// for and awaited, Claude Code launched, its chunks relayed, the
/// closer sent.
///
/// Every failure past the upgrade has no status left to set and says
/// so as a fatal `notification` — the loop's own vocabulary — after
/// which the socket closes:
///
/// - the first message not being the request frame, an agent of
///   another kind, or no prompt at all (Claude Code cannot open a
///   turn without one): the CALLER's;
/// - a continuation the server could not deliver — its own words,
///   verbatim — or one that will not open;
/// - the subprocess failing to start, or a run that ends without
///   producing anything (every healthy run says at least its bill):
///   the server's own.
///
/// An error the run reports mid-stream is HELD, not sent: fatality
/// is finality. A later chunk proves the run outlived it, and it
/// flushes as a NON-fatal `notification` ahead of that chunk, in
/// arrival order; when the stream ends with errors still held, the
/// LAST of them is the run's death — fatal — and the ones before it
/// flush non-fatal ahead of it. Then the estate: if the model had
/// already spoken (any assistant chunk), the harvest follows even the
/// fatal last word, salvaging the progress into the closer; a run
/// that died before the model spoke saved nothing beyond what the
/// caller brought — even a resumed transcript's new prompt line is
/// not progress — and closes with none.
///
/// A stream that ends cleanly — no held error as its last word —
/// closes with THE HARVEST: the session's files swept into the
/// continuation — [`Continuation::read`] keyed by the session id the
/// stream captured (falling back to the resumed one's own) — sent as
/// the closer: raw bytes, chunked.
async fn drive(socket: &mut WebSocket) {
    // The request: the first binary message, the wire's own frame.
    let request = match socket.recv().await {
        Some(Ok(Message::Binary(bytes))) => {
            match agentic_loop_container::request::Request::decode(&bytes) {
                Ok(request) => request,
                Err(error) => {
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

    let Agent::ClaudeCode(agent) = request.agent else {
        fail(
            socket,
            serde_json::json!({
                "kind": "wrong_agent",
                "error": "this container serves the claude_code agent kind",
            }),
        )
        .await;
        return;
    };

    // There must be a prompt: stream-json input mode opens the turn
    // with a user message, so an empty one would hang forever
    // waiting.
    if request.prompt.is_empty() {
        fail(
            socket,
            serde_json::json!({
                "kind": "prompt",
                "error": "a turn needs a prompt",
            }),
        )
        .await;
        return;
    }
    let prompt = request.prompt.clone();

    // The ask, then the wait: the server fetches the continuation
    // from the caller and delivers it on the routes.
    if !send(socket, &Response::FetchContinuation).await {
        return;
    }
    let continuation = match continuation_fetcher::STORE.fetch().await {
        Ok(None) => None,
        Ok(Some(chunks)) => match Continuation::parse(&chunks) {
            Ok(continuation) => Some(continuation),
            Err(error) => {
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
    // The harvest's fallback key: a resumed run knows its session
    // before the stream names it.
    let resumed_session_id = continuation
        .as_ref()
        .map(|continuation| continuation.session_id.clone());

    let stream = match spawn::spawn(agent, continuation, prompt).await {
        Ok(stream) => stream,
        Err(error) => {
            fail(
                socket,
                serde_json::json!({
                    "kind": "spawn",
                    "error": error.to_string(),
                }),
            )
            .await;
            return;
        }
    };
    let mut stream = Box::pin(stream);

    // Whether the run said anything at all, and whether the model
    // spoke — the empty-run check and the salvage criterion.
    let mut any = false;
    let mut progressed = false;
    // Fatality is finality: an error is HELD, not sent — a later
    // chunk proves the run outlived it and flushes it non-fatal, in
    // arrival order; at the stream's end, only the LAST held error
    // is the death itself.
    let mut held: Vec<serde_json::Value> = Vec::new();
    while let Some(item) = stream.next().await {
        any = true;
        match item {
            Ok(chunk) => {
                for message in held.drain(..) {
                    let flushed = Response::Chunk(notification(message, false));
                    if !send(socket, &flushed).await {
                        return;
                    }
                }
                progressed |= assistant_chunk(&chunk);
                if !send(socket, &Response::Chunk(chunk)).await {
                    return;
                }
            }
            Err(error) => held.push(error.message()),
        }
    }
    if !any {
        fail(
            socket,
            serde_json::json!({
                "kind": "empty_run",
                "error": "the run ended without producing anything",
            }),
        )
        .await;
        return;
    }
    match held.pop() {
        None => {
            closer(socket, resumed_session_id).await;
        }
        // Only the LAST error is the stream's death — fatal; the
        // ones before it flush non-fatal, as they would have had
        // anything else followed them. Then the estate: a run the
        // model had spoken in left progress worth resuming, and the
        // harvest salvages it even past the fatal last word. A run
        // that died unspoken saved nothing beyond what the caller
        // brought — a resumed transcript's new prompt line is not
        // progress — and closes with none.
        Some(last) => {
            for message in held.drain(..) {
                let flushed = Response::Chunk(notification(message, false));
                if !send(socket, &flushed).await {
                    return;
                }
            }
            if !send(socket, &Response::Chunk(notification(last, true))).await
            {
                return;
            }
            if progressed {
                closer(socket, resumed_session_id).await;
            }
        }
    }
}

/// The run's last word on success: the session's files swept into
/// the continuation's bytes. A harvest that cannot happen — no
/// record ever named the session and the request resumed nothing,
/// or the sweep or the serializing failed — is a fatal notification
/// instead: the conversation ran, but cannot be resumed.
async fn harvest(
    resumed_session_id: Option<String>,
) -> Result<Vec<u8>, AgenticLoopChunk> {
    let session_id = match spawn::session_id().await.or(resumed_session_id)
    {
        Some(session_id) => session_id,
        None => {
            return Err(notification(
                serde_json::json!({
                    "kind": "harvest",
                    "error": "no record ever named the session",
                }),
                true,
            ));
        }
    };
    let continuation = match Continuation::read(session_id).await {
        Ok(continuation) => continuation,
        Err(error) => {
            return Err(notification(
                serde_json::json!({
                    "kind": "harvest",
                    "error": error.to_string(),
                }),
                true,
            ));
        }
    };
    continuation.tokenize().map_err(|error| {
        notification(
            serde_json::json!({
                "kind": "harvest",
                "error": error.to_string(),
            }),
            true,
        )
    })
}

/// Send the closer: the harvest's bytes as continuation frames, split
/// at [`CHUNK_SIZE`] — or, when the harvest could not happen, the
/// fatal notification it degraded to. Whether the socket stayed up.
async fn closer(
    socket: &mut WebSocket,
    resumed_session_id: Option<String>,
) -> bool {
    match harvest(resumed_session_id).await {
        Ok(bytes) => {
            for piece in bytes.chunks(CHUNK_SIZE) {
                if !send(socket, &Response::Continuation(piece)).await {
                    return false;
                }
            }
            true
        }
        Err(chunk) => send(socket, &Response::Chunk(chunk)).await,
    }
}

/// Whether a chunk is the model speaking — the salvage criterion:
/// any of the six assistant kinds, on ANY thread. A sub-agent
/// speaking is progress the session's files hold, so it earns the
/// harvest exactly as the main thread does.
fn assistant_chunk(chunk: &AgenticLoopChunk) -> bool {
    matches!(
        chunk,
        AgenticLoopChunk::AssistantReasoning(_)
            | AgenticLoopChunk::AssistantTextContent(_)
            | AgenticLoopChunk::AssistantImageContent(_)
            | AgenticLoopChunk::AssistantAudioContent(_)
            | AgenticLoopChunk::AssistantToolCall(_)
            | AgenticLoopChunk::AssistantRefusal(_)
    )
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

/// The install gate every endpoint stands behind: waits out an
/// in-flight install — no answer is knowable before the outcome
/// is — and turns a failed one into the one shared error body.
async fn installed() -> Result<(), (StatusCode, Json<serde_json::Value>)>
{
    match spawn::installed().await {
        Ok(()) => Ok(()),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "kind": "install",
                "error": error,
            })),
        )),
    }
}

/// A message for the running conversation's queue.
///
/// The response IS the fate, and it arrives when the fate is known —
/// taken into the conversation, withdrawn by a dequeue, or outlived
/// by the run. That can be long after the ask; nothing here times
/// anything out. The one HTTP failure is the install's — everything
/// else answers as a fate, even a fate wire dying undecided
/// (missed).
async fn enqueue(
    Json(request): Json<agentic_loop_container::enqueue::Request>,
) -> Result<
    Json<agentic_loop_container::enqueue::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    installed().await?;
    Ok(Json(spawn::enqueue(request.prompt).await))
}

/// One chunk of the continuation into the slot the run will read.
///
/// The body is the bytes verbatim, so nothing can be malformed — the
/// one refusal is a delivery after the settlement (`409`), because
/// anything after a settlement is somebody's bug. The one other HTTP
/// failure is the install's.
async fn continuation_chunk(
    body: axum::body::Bytes,
) -> Result<
    Json<agentic_loop_container::continuation::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    installed().await?;
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
    installed().await?;
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
    installed().await?;
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
/// honest. A `claude_code` agent names no resources, so no ask ever
/// rides this container's socket and no delivery can be answering
/// one: `409`, on all three routes alike (a chunk cannot be
/// malformed — any bytes are one; the endings' JSON is judged by
/// the extractor). The one other HTTP failure is the install's.
async fn resource(
    axum::extract::Path(_identity): axum::extract::Path<String>,
    _body: axum::body::Bytes,
) -> Result<
    Json<agentic_loop_container::resource::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    installed().await?;
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
    installed().await?;
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
    installed().await?;
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
/// The answer arrives once Claude Code has replied to every cancel —
/// however long that takes — and a queue with nothing left to
/// withdraw, or no run at all, answers `empty`. The one HTTP failure
/// is the install's.
async fn dequeue(
    Json(_request): Json<agentic_loop_container::dequeue::Request>,
) -> Result<
    Json<agentic_loop_container::dequeue::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    installed().await?;
    Ok(Json(spawn::dequeue().await))
}
