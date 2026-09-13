//! The `openrouter` agent, as a container.
//!
//! The program an agent container runs for an openrouter agent: an
//! HTTP server on the container's loopback, at the port the SDK's
//! [`diverge_container_proxy_sdk::agent`] module names, that the proxy beside it
//! forwards the provider's asks to. `POST /run` runs a loop — one at
//! a time, the lock held for exactly the stream's life, so a run
//! after the first resumes the conversation and a run beside it is
//! refused — and streams its chunks back as server-sent events;
//! `POST /enqueue` and `POST /dequeue` are the running loop's
//! queue; `GET /schema` is the JSON Schema of the agent value this
//! image accepts; `POST /register` is that value, told once for the
//! container's life before any loop — a run before it is refused. Its tool calls go through the proxy's MCP server;
//! its key comes from the vault the caller holds; the history it
//! resumes from, and leaves behind, is one row in the caller's
//! database, reached through the proxy's loopback pgwire.
//!
//! # Nothing of the proxy's before a request
//!
//! The proxy is not part of this image: the host injects it at
//! runtime, and it may not be up at all until a request comes. So
//! the server binds and waits, and the first thing to touch the
//! proxy — the vault, the database, the MCP session — is `POST /run`
//! itself. Making the [`Client`] is no I/O.
//!
//! # What is an error, and what is not
//!
//! Anything that fails before the loop has said a single thing is a
//! real error — a non-`2xx`, with a JSON reason, and no stream: an
//! agent value that is not an openrouter agent, a vault with no
//! `OPENROUTER_API_KEY`, a database that will not answer, a history
//! that will not open, the first fetch, and the first item of the
//! loop itself, which is pulled before the response is decided. A
//! failure AFTER output is a fatal `notification` chunk, the loop's
//! own last word, then the stream's end. The stream never carries an
//! error of its own: it is chunks, and only chunks.

mod agent;
mod claim;
mod continuation;
mod fetch;
mod history;
mod r#loop;
mod queue;
mod registration;
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

use std::convert::Infallible;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_container_proxy_sdk::Client;
use diverge_container_proxy_sdk::agent::dequeue::Outcome;
use diverge_container_proxy_sdk::agent::enqueue::Fate;
use diverge_container_proxy_sdk::agent::register;
use diverge_container_proxy_sdk::agent::run;
use diverge_provider_sdk::shared::containers::enqueue;
use diverge_provider_sdk::endpoints::containers::agents::run::server::response::{
    AgenticLoopChunk, NotificationChunk,
};
use futures_util::{Stream, StreamExt as _};
use serde_json::Value;

use crate::agent::Agent;
use crate::claim::Claim;
use crate::continuation::Continuation;
use crate::queue::QUEUE;
use crate::r#loop::Item;

/// The vault key the upstream's credential is kept under.
const API_KEY: &str = "OPENROUTER_API_KEY";

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
/// and the proxy is beside it.
async fn serve() {
    let app = axum::Router::new()
        .route("/register", axum::routing::post(register))
        .route("/run", axum::routing::post(run))
        .route("/schema", axum::routing::get(schema))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue))
        .with_state(Arc::new(Client::new()));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", diverge_container_proxy_sdk::agent::port()))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// `POST /run`: the loop.
///
/// Prepares everything the loop needs, starts it, and pulls its FIRST
/// item before answering: an error there is a real error, the status
/// and the reason, with no stream begun. Then the stream — the first
/// chunk, then every chunk as it comes, each history at rest saved as
/// it is reached, and a failure after the first chunk a fatal
/// notification.
///
/// The agent is the registered one, and a run before registration is
/// refused (`409`) before anything else. One run at a time: the [`Claim`] is taken next and refused with
/// `409` while another holds it, and it lives exactly as long as the
/// stream — released on every refusal below, and otherwise captured
/// into the stream, so it drops with it, finished or abandoned. The
/// queue is opened for this run right behind the claim; every
/// refusal before the loop exists closes it again, so a message
/// waiting for a loop that will never look is missed honestly. Once
/// the loop exists its own guard closes the queue however the run
/// ends. The history comes from the cache when this container has
/// loaded or saved one, and from the row only the first time.
async fn run(
    State(client): State<Arc<Client>>,
    Json(request): Json<run::request::Request>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Refusal> {
    let Some(agent) = registration::registered() else {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "unregistered",
                "error": "no agent has been registered",
            })),
        ));
    };
    // Not a refusal of the kind below: the running loop owns the
    // queue, and this ask changes nothing.
    let Some(claim) = Claim::take() else {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "busy",
                "error": "a run is in progress",
            })),
        ));
    };
    let generation = QUEUE.open().await;

    let api_key = match client.vault_get(API_KEY).await {
        Ok(Some(bytes)) => match String::from_utf8(bytes.to_vec()) {
            Ok(api_key) => api_key,
            Err(_) => {
                return Err(refuse(
                generation,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({
                        "kind": "api_key",
                        "error": format!("{API_KEY} is not UTF-8"),
                    }),
                )
                .await);
            }
        },
        Ok(None) => {
            return Err(refuse(
                generation,
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "kind": "api_key",
                    "error": format!("the vault holds no {API_KEY}"),
                }),
            )
            .await);
        }
        Err(error) => {
            return Err(refuse(
                generation,
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "kind": "api_key",
                    "error": error.to_string(),
                }),
            )
            .await);
        }
    };

    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&client.postgres_url())
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            return Err(refuse(
                generation,
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "kind": "continuation",
                    "error": error.to_string(),
                }),
            )
            .await);
        }
    };
    let continuation = match history::cached().await {
        Some(continuation) => continuation,
        None => match Continuation::load(&pool).await {
            Ok(continuation) => {
                history::remember(continuation.clone()).await;
                continuation
            }
            Err(error) => {
                return Err(refuse(generation, StatusCode::INTERNAL_SERVER_ERROR, error.message()).await);
            }
        },
    };

    let mut items = match r#loop::r#loop(
        &client,
        &api_key,
        agent,
        continuation,
        request.prompt,
        generation,
    )
    .await
    {
        Ok(items) => items,
        Err(error) => {
            return Err(refuse(generation, StatusCode::INTERNAL_SERVER_ERROR, error.message()).await);
        }
    };

    // The first item decides. Pulled here, before the status is
    // chosen: an error is the status; a chunk is the stream's first
    // event; a rest is saved like any other; nothing at all is an
    // empty stream, the loop over before it spoke. The loop's own
    // guard has the queue from here on, whichever it is.
    let first = match items.next().await {
        None => None,
        Some(Ok(Item::Chunk(chunk))) => Some(chunk),
        Some(Ok(Item::Rest(history))) => {
            if let Err(error) = history.save(&pool).await {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error.message())));
            }
            history::remember(Some(history)).await;
            None
        }
        Some(Err(error)) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error.message())));
        }
    };

    Ok(Sse::new(async_stream::stream! {
        // The lock lives here now: it drops when the stream does.
        let _claim = claim;
        let mut items = items;
        if let Some(chunk) = first {
            yield Ok(event(&chunk));
        }
        while let Some(item) = items.next().await {
            match item {
                Ok(Item::Chunk(chunk)) => {
                    yield Ok(event(&chunk));
                }
                Ok(Item::Rest(history)) => {
                    if let Err(error) = history.save(&pool).await {
                        yield Ok(event(&notification(error.message(), true)));
                        return;
                    }
                    history::remember(Some(history)).await;
                }
                Err(error) => {
                    yield Ok(event(&notification(error.message(), true)));
                    return;
                }
            }
        }
    }))
}

/// `POST /register`: the agent, once, for the container's life.
///
/// A value this image will not take is `400`; an agent already
/// registered is `409`, whatever the second carries — the agent
/// never changes. `204` is the agent held.
async fn register(Json(request): Json<register::request::Request>) -> Result<StatusCode, Refusal> {
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
    match registration::register(agent) {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "registered",
                "error": "the agent is registered, and it never changes",
            })),
        )),
    }
}

/// `GET /schema`: what the agent value may be — the JSON Schema of
/// [`Agent`], derived from the type the loop reads, so the two cannot
/// disagree.
async fn schema() -> Json<schemars::Schema> {
    Json(schemars::schema_for!(Agent))
}

/// `POST /enqueue`: a message for the running loop's queue.
///
/// The response IS the fate, and it arrives when the fate is known —
/// taken into the conversation, withdrawn by a dequeue, or outlived
/// by the run. That can be long after the ask; nothing here times
/// anything out. The one failure with no fate to report — the fate
/// channel dying, which the loop's close guard exists to prevent —
/// answers as HTTP does, with a status.
async fn enqueue(Json(request): Json<enqueue::request::Request>) -> Result<Json<Fate>, Refusal> {
    let fate = QUEUE.enqueue(request.prompt).await;
    match fate.await {
        Ok(fate) => Ok(Json(fate)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "kind": "fate_lost",
                "error": "the message's fate was never decided",
            })),
        )),
    }
}

/// `POST /dequeue`: clear the running loop's queue.
///
/// Naive, deliberately: whatever is pending is withdrawn — each
/// message's own `/enqueue` answers `dequeued` — and a queue with
/// nothing pending, closed or not, answers `empty`. The body, `{}`,
/// carries nothing and is not read.
async fn dequeue() -> Json<Outcome> {
    if QUEUE.dequeue().await {
        Json(Outcome::Dequeued)
    } else {
        Json(Outcome::Empty)
    }
}

/// A refusal before the loop exists: this run's queue closed, then
/// the status and the reason. The claim is the caller's local, and
/// drops on its return.
async fn refuse(generation: u64, status: StatusCode, message: Value) -> Refusal {
    QUEUE.close(generation).await;
    (status, Json(message))
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
