//! The `claude_code` agent, as a container.
//!
//! The program an agent container runs for a Claude Code agent: an
//! HTTP server on the container's loopback, at the port the SDK's
//! [`container_proxy::agent`] module names, that the proxy beside it
//! forwards the provider's asks to. `POST /run` runs the one loop the
//! container serves — a Claude Code subprocess behind [`spawn`] — and
//! streams its chunks back as server-sent events — one at a time,
//! the lock held until the run is settled, so a run after the first
//! resumes the same session and a run beside it is refused;
//! `POST /enqueue` and `POST /dequeue` are the running loop's queue, which Claude Code
//! holds and this container writes to; `GET /schema` is the JSON
//! Schema of the agent value this image accepts. Claude Code's tool
//! calls go through the proxy's MCP server, whose URL it is handed on
//! its argv; the session it resumes from, and leaves behind, is one
//! row in the caller's database, reached through the proxy's loopback
//! pgwire — read once, the first run, and written at the end of
//! every run. The one thing cached is the session id: known once, it
//! never changes, and the files it names are on disk for good, so a
//! later run neither reads the row nor writes a file.
//!
//! # Nothing of the proxy's before a request
//!
//! The proxy is not part of this image: the host injects it at
//! runtime, and it may not be up at all until a request comes. So
//! the server binds and waits — the one thing it does at startup is
//! install Claude Code, from npm, not the proxy — and the first
//! thing to touch the proxy is `POST /run` itself: the database,
//! then Claude Code's own MCP client. Making the [`Client`] is no
//! I/O.
//!
//! # What is an error, and what is not
//!
//! Anything that fails before the run has said a single thing is a
//! real error — a non-`2xx`, with a JSON reason, and no stream: a
//! second run, an install that failed, an agent value that is not a
//! Claude Code agent, an empty prompt, a database that will not
//! answer, a session that will not open or whose files will not
//! write, a subprocess that will not start, and the first item of
//! the stream itself, which is pulled
//! before the response is decided. After that, FATALITY IS FINALITY:
//! an error record is held, not sent; a later chunk proves the run
//! outlived it and flushes it as a non-fatal `notification` ahead of
//! that chunk; when the stream ends with errors still held, the last
//! of them is the run's death, the fatal final chunk. The stream
//! never carries an error of its own: it is chunks, and only chunks.

mod agent;
mod claim;
mod continuation;
// The wire module carries Claude Code's COMPLETE stdout vocabulary,
// which is more than the conversion consumes — a field parsed and
// never read is the completeness, not dead code.
#[allow(dead_code)]
mod response;
mod spawn;

use std::convert::Infallible;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_container_proxy_sdk::Client;
use diverge_provider_sdk::container_proxy;
use diverge_provider_sdk::container_proxy::agent::dequeue::Outcome;
use diverge_provider_sdk::container_proxy::agent::enqueue::Fate;
use diverge_provider_sdk::container_proxy::run_loop;
use diverge_provider_sdk::shared::containers::enqueue;
use diverge_provider_sdk::shared::containers::run_loop::response::{
    AgenticLoopChunk, NotificationChunk,
};
use futures_util::{Stream, StreamExt as _};
use serde_json::Value;
use sqlx::PgPool;

use crate::agent::Agent;
use crate::claim::Claim;
use crate::continuation::Continuation;

/// A refusal: the status, and a JSON reason.
type Refusal = (StatusCode, Json<Value>);

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(serve());
}

/// The server, for the container's life.
///
/// Bound on the loopback: the proxy is the only thing that dials it,
/// and the proxy is beside it. The install begins the moment the
/// process does — requests or none — and every endpoint awaits the
/// same memoized outcome.
async fn serve() {
    tokio::spawn(async {
        let _ = spawn::installed().await;
    });

    let app = axum::Router::new()
        .route("/run", axum::routing::post(run))
        .route("/schema", axum::routing::get(schema))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue))
        .with_state(Arc::new(Client::new()));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", container_proxy::agent::port()))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// `POST /run`: the run.
///
/// Refused, in order, for what is knowable before the stream: a run
/// beside one in progress (`409` — the [`Claim`], released on every
/// refusal below and otherwise by the run's settlement, so the next
/// run finds clean locks); Claude Code failing to install (`500`,
/// and the same on every endpoint, forever — a request during the
/// install simply waits for the outcome); an agent value that is not
/// this image's (`400`); an empty prompt (`400` — stream-json input
/// opens the turn with a user message, and an empty one would hang
/// forever waiting); a database that will not answer, or a session
/// row that will not open (`500`); a subprocess that will not start
/// (`500`); and the stream's FIRST item, pulled before the response
/// is decided — an error record there is the request's own failure
/// (`500`, in the record's own words), and a stream with nothing at
/// all is `empty_run` (every healthy run says at least its bill).
///
/// Then the stream: the first chunk, then every chunk as it comes,
/// with the held-error relay the crate doc states. And the estate,
/// when the stream ends: on a clean end, or a fatal end after the
/// model had spoken (any assistant chunk — a sub-agent's counts),
/// the session's files are harvested and saved as the continuation;
/// a harvest or a save that cannot happen is a fatal notification
/// instead, because the conversation ran but cannot be resumed. A
/// run that died before the model spoke saved nothing beyond what
/// the caller brought — even a resumed transcript's new prompt line
/// is not progress — and leaves the row as it was.
async fn run(
    State(client): State<Arc<Client>>,
    Json(request): Json<run_loop::request::Request>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Refusal> {
    let Some(claim) = Claim::take() else {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "busy",
                "error": "a run is in progress",
            })),
        ));
    };
    installed().await?;

    let agent: Agent = match serde_json::from_value(request.agent) {
        Ok(agent) => agent,
        Err(error) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "kind": "agent",
                    "error": error.to_string(),
                })),
            ));
        }
    };
    if request.prompt.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "kind": "prompt",
                "error": "a turn needs a prompt",
            })),
        ));
    }

    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&client.postgres_url())
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "kind": "continuation",
                    "error": error.to_string(),
                })),
            ));
        }
    };
    // The session: known already — its files on disk, nothing to
    // read and nothing to write — or, the first time, whatever the
    // row holds: nothing, a fresh conversation; or a continuation,
    // whose files are laid down now, once, and whose id is then
    // known for good.
    let session_id = match spawn::session_id().await {
        Some(session_id) => Some(session_id),
        None => match Continuation::load(&pool).await {
            Ok(None) => None,
            Ok(Some(continuation)) => {
                if let Err(error) = continuation.write().await {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "kind": "continuation",
                            "error": error.to_string(),
                        })),
                    ));
                }
                spawn::remember(continuation.session_id.clone()).await;
                Some(continuation.session_id)
            }
            Err(error) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error.message())));
            }
        },
    };

    let stream = match spawn::spawn(agent, session_id, request.prompt, claim).await {
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

    // The first item decides. Pulled here, before the status is
    // chosen: an error record is the request's own failure; nothing
    // at all is a run that never spoke; a chunk is the stream's
    // first event.
    let first = match stream.next().await {
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "kind": "empty_run",
                    "error": "the run ended without producing anything",
                })),
            ));
        }
        Some(Err(error)) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error.message())));
        }
        Some(Ok(chunk)) => chunk,
    };

    Ok(Sse::new(async_stream::stream! {
        let mut stream = stream;
        // Whether the model spoke — the salvage criterion.
        let mut progressed = assistant_chunk(&first);
        // Fatality is finality: an error is HELD, not sent — a later
        // chunk proves the run outlived it and flushes it non-fatal,
        // in arrival order; at the stream's end, only the LAST held
        // error is the death itself.
        let mut held: Vec<Value> = Vec::new();
        yield Ok(event(&first));
        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => {
                    for message in held.drain(..) {
                        yield Ok(event(&notification(message, false)));
                    }
                    progressed |= assistant_chunk(&chunk);
                    yield Ok(event(&chunk));
                }
                Err(error) => held.push(error.message()),
            }
        }
        match held.pop() {
            None => {
                if let Err(chunk) = harvest(&pool).await {
                    yield Ok(event(&chunk));
                }
            }
            // Only the LAST error is the stream's death — fatal; the
            // ones before it flush non-fatal, as they would have had
            // anything else followed them. Then the estate, if the
            // model had spoken.
            Some(last) => {
                for message in held.drain(..) {
                    yield Ok(event(&notification(message, false)));
                }
                yield Ok(event(&notification(last, true)));
                if progressed {
                    if let Err(chunk) = harvest(&pool).await {
                        yield Ok(event(&chunk));
                    }
                }
            }
        }
    }))
}

/// `GET /schema`: what the agent value may be — the JSON Schema of
/// [`Agent`], derived from the type the run reads, so the two cannot
/// disagree.
async fn schema() -> Json<schemars::Schema> {
    Json(schemars::schema_for!(Agent))
}

/// `POST /enqueue`: a message for the running conversation's queue.
///
/// The response IS the fate, and it arrives when the fate is known —
/// taken into the conversation, withdrawn by a dequeue, or outlived
/// by the run. That can be long after the ask; nothing here times
/// anything out. The one HTTP failure is the install's — everything
/// else answers as a fate, even a fate wire dying undecided
/// (missed).
async fn enqueue(Json(request): Json<enqueue::request::Request>) -> Result<Json<Fate>, Refusal> {
    installed().await?;
    Ok(Json(spawn::enqueue(request.prompt).await))
}

/// `POST /dequeue`: clear the running conversation's queue.
///
/// The answer arrives once Claude Code has replied to every cancel —
/// however long that takes — and a queue with nothing left to
/// withdraw, or no run at all, answers `empty`. The body, `{}`,
/// carries nothing and is not read. The one HTTP failure is the
/// install's.
async fn dequeue() -> Result<Json<Outcome>, Refusal> {
    installed().await?;
    Ok(Json(spawn::dequeue().await))
}

/// The install gate every endpoint stands behind: waits out an
/// in-flight install — no answer is knowable before the outcome
/// is — and turns a failed one into the one shared error body.
async fn installed() -> Result<(), Refusal> {
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

/// The run's estate: the session's files swept into the continuation
/// and saved as the row. Keyed by the session id — known from the
/// row, or from the first record that named it. A harvest that
/// cannot happen — no record ever named the session and the request
/// resumed nothing, or the sweep or the save failed — is the fatal
/// notification the stream ends on instead.
async fn harvest(pool: &PgPool) -> Result<(), AgenticLoopChunk> {
    let session_id = match spawn::session_id().await {
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
    continuation
        .save(pool)
        .await
        .map_err(|error| notification(error.message(), true))
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

/// One chunk, as the event that carries it: its JSON as the data.
///
/// Plain data never fails to serialize, so a chunk that does names a
/// bug rather than a circumstance; it goes out as a fatal
/// notification saying so, since the stream has no other way to
/// speak.
fn event(chunk: &AgenticLoopChunk) -> Event {
    match serde_json::to_string(chunk) {
        Ok(data) => Event::default().data(data),
        Err(error) => {
            let fatal = notification(
                serde_json::json!({
                    "kind": "chunk",
                    "error": format!("a chunk would not serialize: {error}"),
                }),
                true,
            );
            Event::default().data(serde_json::to_string(&fatal).unwrap_or_default())
        }
    }
}

/// A notification chunk, its fatality the caller's verdict.
fn notification(message: Value, is_fatal: bool) -> AgenticLoopChunk {
    AgenticLoopChunk::Notification(NotificationChunk {
        r#type: Default::default(),
        is_fatal,
        message,
        meta: None,
    })
}
