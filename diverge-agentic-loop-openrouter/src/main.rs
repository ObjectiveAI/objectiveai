//! The `openrouter` agent, as a container.
//!
//! The program an agent container runs for an agent whose `upstream`
//! is `openrouter`: an HTTP server on the container's loopback, at the
//! port the SDK's [`agent`] module names, that the proxy beside it
//! forwards the provider's asks to. `POST /run` runs the one loop the
//! container serves and streams its chunks back as server-sent
//! events; `POST /enqueue` and `POST /dequeue` are the running loop's
//! queue; `GET /schema` would be the agent's schema, which this image
//! does not state. Its tool calls go through the proxy's MCP server;
//! its key comes from the vault the caller holds; the history it
//! resumes from, and leaves behind, is one row in the caller's
//! database, reached through the proxy's loopback pgwire.
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

mod continuation;
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

use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_container_proxy_sdk::Client;
use diverge_provider_sdk::container_proxy::agent;
use diverge_provider_sdk::container_proxy::agent::dequeue::Outcome;
use diverge_provider_sdk::container_proxy::agent::enqueue::Fate;
use diverge_provider_sdk::container_proxy::run_loop;
use diverge_provider_sdk::endpoints::containers::agents::agent::openrouter;
use diverge_provider_sdk::shared::containers::enqueue;
use diverge_provider_sdk::shared::containers::run_loop::response::{
    AgenticLoopChunk, NotificationChunk,
};
use futures_util::{Stream, StreamExt as _};
use serde_json::Value;

use crate::continuation::Continuation;
use crate::queue::QUEUE;
use crate::r#loop::Item;

/// The vault key the upstream's credential is kept under.
const API_KEY: &str = "OPENROUTER_API_KEY";

/// Whether the container's one run has arrived.
///
/// A container serves one loop, ever: the history it resumes from is
/// loaded once and its queue is one run's. A second `/run` is refused
/// with `409`, and the flag is never cleared.
static CLAIMED: AtomicBool = AtomicBool::new(false);

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
        .route("/run", axum::routing::post(run))
        .route("/schema", axum::routing::get(schema))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue))
        .with_state(Arc::new(Client::new()));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", agent::port()))
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
/// notification. Every refusal before the loop exists also closes
/// the queue: the container is spent, and a message waiting for a
/// loop that will never look is missed honestly instead. Once the
/// loop exists its own guard closes the queue however the run ends —
/// the stream dropped by a caller that left included.
async fn run(
    State(client): State<Arc<Client>>,
    Json(request): Json<run_loop::request::Request>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Refusal> {
    if CLAIMED.swap(true, Ordering::SeqCst) {
        // Not a refusal of the kind below: the running loop owns the
        // queue, and this ask changes nothing.
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "claimed",
                "error": "this container serves one run, and it has already begun",
            })),
        ));
    }

    let agent: openrouter::Agent = match serde_json::from_value(request.agent) {
        Ok(agent) => agent,
        Err(error) => {
            return Err(refuse(
                StatusCode::BAD_REQUEST,
                serde_json::json!({
                    "kind": "agent",
                    "error": error.to_string(),
                }),
            )
            .await);
        }
    };

    let api_key = match client.vault_get(API_KEY).await {
        Ok(Some(bytes)) => match String::from_utf8(bytes.to_vec()) {
            Ok(api_key) => api_key,
            Err(_) => {
                return Err(refuse(
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
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "kind": "continuation",
                    "error": error.to_string(),
                }),
            )
            .await);
        }
    };
    let continuation = match Continuation::load(&pool).await {
        Ok(continuation) => continuation,
        Err(error) => {
            return Err(refuse(StatusCode::INTERNAL_SERVER_ERROR, error.message()).await);
        }
    };

    let mut items = match r#loop::r#loop(
        &client,
        &api_key,
        agent,
        continuation,
        request.prompt,
    )
    .await
    {
        Ok(items) => items,
        Err(error) => {
            return Err(refuse(StatusCode::INTERNAL_SERVER_ERROR, error.message()).await);
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
            None
        }
        Some(Err(error)) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error.message())));
        }
    };

    Ok(Sse::new(async_stream::stream! {
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
                }
                Err(error) => {
                    yield Ok(event(&notification(error.message(), true)));
                    return;
                }
            }
        }
    }))
}

/// `GET /schema`: what the agent value may be — which this image does
/// not state. A schema is a courtesy, not an obligation, and the
/// refusal says so.
async fn schema() -> Refusal {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "kind": "agent_schema",
            "error": "this image states no schema",
        })),
    )
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

/// A refusal before the loop exists: the queue closed, then the
/// status and the reason.
async fn refuse(status: StatusCode, message: Value) -> Refusal {
    QUEUE.close().await;
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
