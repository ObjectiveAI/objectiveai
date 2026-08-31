//! The `claude_code` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `claude_code`, per the Container section of the
//! provider specification: one POST at `/` on port 8080 carries the
//! caller's request JSON in, and the answer is a server-sent event
//! stream, each event one item of the container response vocabulary
//! — the run itself a Claude Code subprocess behind [`spawn`]. MCP
//! is asked on port 8081, Postgres opened to port 8082. Beside the
//! run, the queue's two verbs: `POST /enqueue` and `POST /dequeue`,
//! per the SDK's `agentic_loop_container` module — the caller's way
//! into the conversation already running: Claude Code holds the
//! queue, and this container holds the writer. `POST /resource` is
//! served because the surface has it, and answers honestly: a
//! `claude_code` agent names no resources, so every delivery is
//! unrequested.

mod continuation;
// The wire module carries Claude Code's COMPLETE stdout vocabulary,
// which is more than the conversion consumes — a field parsed and
// never read is the completeness, not dead code.
#[allow(dead_code)]
mod response;
mod spawn;

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_provider_sdk::agentic_loop_container;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::Agent;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::{
    AgenticLoopChunk, ContinuationChunk, NotificationChunk,
};
use futures_util::{Stream, StreamExt as _};

use crate::continuation::Continuation;

/// The loop port of the Container section of the provider
/// specification: where the server POSTs the request in.
const PORT: u16 = 8080;

/// Whether the container's one request has arrived.
///
/// A container is one run: its queue, its MCP session, its
/// filesystem are all one conversation's, and a second request would
/// share all of them with the first. So the FIRST request claims the
/// container for good — an atomic swap, so two arrivals a nanosecond
/// apart resolve to exactly one winner — and everything after it,
/// concurrent or later, is refused with `409` before anything else
/// is judged: a first request that fails every later check has still
/// spent the container, because "one request" is a fact about
/// arrivals, not about merit. (A body that never parsed as the
/// request type never arrived as one — the extractor's `400` comes
/// first and claims nothing.)
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
        .route("/", axum::routing::post(serve))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue))
        .route("/resource", axum::routing::post(resource));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// One request, one stream.
///
/// The caller's JSON arrives as the SDK's own request type, and
/// every chunk the run produces leaves as one SSE event carrying
/// that chunk's JSON. The failures divide by whose they are:
///
/// - A request after the first is the CALLER's error, and the first
///   error checked: `409`, the container is [`CLAIMED`].
/// - Claude Code failing to INSTALL — the harness fetches it at
///   startup; see [`spawn::installed`] — is the server's own,
///   checked right after the claim (arrival still spends the
///   container): `500`, and the same on every other endpoint,
///   forever. A request during the install simply waits for the
///   outcome.
/// - An agent of another kind, a prompt this container cannot yet
///   speak (anything richer than text, or nothing at all — Claude
///   Code cannot open a turn without a prompt), or a continuation
///   token that will not open: the CALLER's error, `400`.
/// - The subprocess failing to start is the server's own: `500`.
/// - An error-typed record BEFORE the run's first chunk — the
///   first-item contract below — answers as HTTP with the record's
///   own status and body (the stream error's `status`/`message`);
///   so does a run that dies producing nothing at all, since every
///   healthy run says at least its bill.
/// - An error after the stream began cannot change the status that
///   already left — and its severity is not knowable on arrival, so
///   it is HELD, not yielded: fatality is finality. A later chunk
///   proves the run outlived it, and it flushes as a NON-fatal
///   `notification` ahead of that chunk, in arrival order; when the
///   stream ends with errors still held, the LAST of them is the
///   run's death — fatal — and the ones before it flush non-fatal
///   ahead of it. Then the estate: if the model had already spoken
///   (any assistant chunk), the harvest follows even the fatal last
///   word, salvaging the progress into a continuation; a run that
///   died before the model spoke saved nothing beyond what the
///   caller brought — even a resumed transcript's new prompt line
///   is not progress — and yields none.
///
/// A stream that ends cleanly — no held error as its last word —
/// closes with THE HARVEST: the session's files swept into a
/// continuation token — [`Continuation::read`] keyed by the session
/// id the stream captured (falling back to the resumed token's
/// own) — yielded as the final `continuation` chunk.
async fn serve(
    Json(request): Json<agentic_loop_container::request::Request>,
) -> Result<
    Sse<impl Stream<Item = Result<Event, axum::Error>>>,
    (StatusCode, Json<serde_json::Value>),
> {
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

    let Agent::ClaudeCode(agent) = request.agent else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "kind": "wrong_agent",
                "error": "this container serves the claude_code agent kind",
            })),
        ));
    };

    // Text-only for now: every block must be text, and there must be
    // at least one — stream-json input mode opens the turn with a
    // user message, so an empty prompt would hang forever waiting.
    let Some(prompt) = prompt_text(&request.prompt) else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "kind": "prompt",
                "error": "this container speaks text prompts only, and needs one",
            })),
        ));
    };

    let continuation = match request
        .continuation
        .as_deref()
        .map(Continuation::parse)
        .transpose()
    {
        Ok(continuation) => continuation,
        Err(error) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "kind": "continuation",
                    "error": error.to_string(),
                })),
            ));
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
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "kind": "spawn",
                    "error": error.to_string(),
                })),
            ));
        }
    };
    let mut stream = Box::pin(stream);

    // The first-item contract: the run's first word decides whether
    // this answer is a stream at all. An error-typed record before
    // the first chunk is the request's own failure, as HTTP; a run
    // that ends before saying anything said nothing because it died —
    // every healthy run says at least its bill.
    let first = match stream.next().await {
        Some(Ok(chunk)) => chunk,
        Some(Err(error)) => {
            return Err((
                StatusCode::from_u16(error.status())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error.message()),
            ));
        }
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "kind": "empty_run",
                    "error": "the run ended without producing anything",
                })),
            ));
        }
    };

    Ok(Sse::new(async_stream::stream! {
        // Whether the model has spoken — the salvage criterion.
        let mut progressed = assistant_chunk(&first);
        yield event(first);
        // Fatality is finality: an error is HELD, not yielded — a
        // later chunk proves the run outlived it and flushes it
        // non-fatal, in arrival order; at the stream's end, only
        // the LAST held error is the death itself.
        let mut held: Vec<serde_json::Value> = Vec::new();
        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => {
                    for message in held.drain(..) {
                        yield event(notification(message, false));
                    }
                    progressed |= assistant_chunk(&chunk);
                    yield event(chunk);
                }
                Err(error) => held.push(error.message()),
            }
        }
        match held.pop() {
            None => yield event(harvest(resumed_session_id).await),
            // Only the LAST error is the stream's death — fatal; the
            // ones before it flush non-fatal, as they would have had
            // anything else followed them. Then the estate: a run
            // the model had spoken in left progress worth resuming,
            // and the harvest salvages it even past the fatal last
            // word. A run that died unspoken saved nothing beyond
            // what the caller brought — a resumed transcript's new
            // prompt line is not progress — and yields none.
            Some(last) => {
                for message in held.drain(..) {
                    yield event(notification(message, false));
                }
                yield event(notification(last, true));
                if progressed {
                    yield event(harvest(resumed_session_id).await);
                }
            }
        }
    }))
}

/// The whole prompt as text: every block's text, joined by blank
/// lines — or [`None`] the moment any block is richer than text, or
/// when there are no blocks at all.
fn prompt_text(prompt: &[rmcp::model::ContentBlock]) -> Option<String> {
    if prompt.is_empty() {
        return None;
    }
    let mut texts = Vec::with_capacity(prompt.len());
    for block in prompt {
        texts.push(block.as_text()?.text.as_str());
    }
    Some(texts.join("\n\n"))
}

/// The run's last word on success: the session's files swept into a
/// continuation token. A harvest that cannot happen — no record ever
/// named the session and the request resumed nothing, or the sweep
/// or the tokenizing failed — is a fatal notification instead: the
/// conversation ran, but cannot be resumed.
async fn harvest(
    resumed_session_id: Option<String>,
) -> AgenticLoopChunk {
    let session_id = match spawn::session_id().await.or(resumed_session_id)
    {
        Some(session_id) => session_id,
        None => {
            return notification(
                serde_json::json!({
                    "kind": "harvest",
                    "error": "no record ever named the session",
                }),
                true,
            );
        }
    };
    let continuation = match Continuation::read(session_id).await {
        Ok(continuation) => continuation,
        Err(error) => {
            return notification(
                serde_json::json!({
                    "kind": "harvest",
                    "error": error.to_string(),
                }),
                true,
            );
        }
    };
    match continuation.tokenize() {
        Ok(token) => AgenticLoopChunk::Continuation(ContinuationChunk {
            r#type: Default::default(),
            continuation: token,
            meta: None,
        }),
        Err(error) => notification(
            serde_json::json!({
                "kind": "harvest",
                "error": error.to_string(),
            }),
            true,
        ),
    }
}

/// Whether a chunk is the model speaking — the salvage criterion:
/// any of the six assistant kinds.
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

/// One chunk as one SSE event, carrying the chunk's JSON — the
/// [`Chunk`](agentic_loop_container::response::Response::Chunk) arm
/// of the container response item, which serializes as the bare
/// chunk. (The union's other arm is the resource ask, which this
/// container never sends.)
fn event(chunk: AgenticLoopChunk) -> Result<Event, axum::Error> {
    Event::default()
        .json_data(agentic_loop_container::response::Response::Chunk(chunk))
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

/// A resource delivery, for a container that never asks for one.
///
/// The route is the surface's, so it is served; the answer is
/// honest. A `claude_code` agent names no resources, so no ask ever
/// rides this container's stream and no delivery can be answering
/// one: `409` — after the body itself is judged, so a malformed
/// POST is still its sender's first problem (`400`). The one other
/// HTTP failure is the install's.
async fn resource(
    body: axum::body::Bytes,
) -> Result<
    Json<agentic_loop_container::resource::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    installed().await?;
    if let Err(error) =
        agentic_loop_container::resource::Request::decode(&body)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "kind": "malformed",
                "error": error.to_string(),
            })),
        ));
    }
    Err((
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "kind": "unrequested",
            "error": "this container never asks for resources",
        })),
    ))
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
