//! The `eliza` agent, as a container.
//!
//! The program an agent container runs for an Eliza (elizaOS) agent:
//! an HTTP server on the container's loopback, at the port the SDK's
//! [`diverge_container_proxy_sdk::agent`] module names, that the proxy beside it
//! forwards the provider's asks to. `POST /run` runs a loop — one at
//! a time, the lock held until the run is settled, so a run after
//! the first resumes the same conversation and a run beside it is
//! refused — and streams its chunks back as server-sent events;
//! `POST /enqueue` and `POST /dequeue` are the running loop's queue,
//! taken at each turn's end; `GET /schema` is the JSON Schema of the
//! agent value this image accepts; `POST /register` is that value,
//! told once for the container's life before any run — a run before
//! it is refused. The run itself is [`run::run`]: the Node entry
//! over `@elizaos/core` spawned per run and driven over JSON lines,
//! its tools the caller's MCP servers through the proxy (the entry's
//! `diverge` plugin), its secrets the caller's vault, its memory the
//! caller's database — Eliza's own tables, reached through the
//! proxy's loopback pgwire, plus the one lineage row this program
//! keeps. The design is `reports/4.md`; the settled decisions are
//! `HARNESS.md`.
//!
//! # Nothing of the proxy's before a request
//!
//! The proxy is not part of this image: the host injects it at
//! runtime, and it may not be up at all until a request comes. So
//! the server binds and waits, and the first thing to touch the
//! proxy is `POST /run` itself: the database, the vault, then the
//! entry's own MCP client. Making the [`Client`] is no I/O.
//!
//! # What is an error, and what is not
//!
//! Anything that fails before the run has said a single thing is a
//! real error — a non-`2xx`, with a JSON reason, and no stream: a run
//! beside one streaming, an agent value that is not an Eliza agent,
//! an empty prompt, a database that will not answer, a secret the
//! vault does not hold, a plugin that will not install, an entry that
//! will not start or dies before it is ready, and the first item of
//! the stream itself, which is pulled before the response is decided.
//! A failure AFTER output is a fatal `notification` chunk, the run's
//! own last words, then the stream's end. The stream never carries an
//! error of its own: it is chunks, and only chunks.

mod agent;
mod claim;
mod lineage;
mod plugins;
mod queue;
mod registration;
mod run;
mod settings;
mod vault;

use std::convert::Infallible;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_container_proxy_sdk::Client;
use diverge_container_proxy_sdk::agent::dequeue::Outcome;
use diverge_container_proxy_sdk::agent::enqueue::Fate;
use diverge_provider_sdk::shared::containers::enqueue;
use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use futures_util::{Stream, StreamExt as _};
use serde_json::Value;

use crate::agent::Agent;
use crate::claim::Claim;
use crate::queue::QUEUE;
use crate::run::notification;

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

/// `POST /run`: the run.
///
/// Refused, in order, for what is knowable before the stream: a run
/// before the agent is registered (`409`); a run beside one streaming
/// (`409` — the [`Claim`], released on every refusal below and
/// otherwise by the run's settlement; a request that lands while a
/// settlement is still running waits for it and is never refused);
/// an empty prompt (`400` — a turn needs one); a database that will
/// not answer (`500`); and the run's FIRST item, pulled before the
/// response is decided — the run's one `Err` is the request's own
/// failure (`500`, in its own words), and a run with nothing to say
/// is `empty_run`. The queue is opened for this run right behind the
/// claim; every refusal before the run exists closes it again, so a
/// message waiting for a loop that will never look is missed
/// honestly. Then the stream: the first chunk, then every chunk as it
/// comes.
async fn run(
    State(client): State<Arc<Client>>,
    Json(request): Json<diverge_container_proxy_sdk::agent::run::request::Request>,
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
    let Ok(claim) = Claim::take().await else {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "busy",
                "error": "a run is in progress",
            })),
        ));
    };
    let generation = QUEUE.open().await;

    if request.prompt.is_empty() {
        return Err(refuse(
            generation,
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "kind": "prompt",
                "error": "a turn needs a prompt",
            }),
        )
        .await);
    }

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
                    "kind": "lineage",
                    "error": error.to_string(),
                }),
            )
            .await);
        }
    };

    let mut items = Box::pin(run::run(
        client,
        pool,
        agent,
        request.prompt,
        generation,
        claim,
    ));

    // The first item decides. Pulled here, before the status is
    // chosen: the run's one error is the status; a chunk is the
    // stream's first event; nothing at all is a run that never spoke.
    // The teardown has the claim and the queue from here on.
    let first = match items.next().await {
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
        let mut items = items;
        yield Ok(event(&first));
        while let Some(item) = items.next().await {
            match item {
                Ok(chunk) => {
                    yield Ok(event(&chunk));
                }
                // By construction the run's `Err` is only ever its
                // first item; a later one would be a bug, and is
                // spoken as the fatal last word rather than lost.
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
async fn register(
    Json(request): Json<diverge_container_proxy_sdk::agent::register::request::Request>,
) -> Result<StatusCode, Refusal> {
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
/// [`Agent`], derived from the type the run reads, so the two cannot
/// disagree.
async fn schema() -> Json<schemars::Schema> {
    Json(schemars::schema_for!(Agent))
}

/// `POST /enqueue`: a message for the running conversation's queue.
///
/// The response IS the fate, and it arrives when the fate is known —
/// taken as the next turn's input, withdrawn by a dequeue, or
/// outlived by the run. That can be long after the ask; nothing here
/// times anything out. The one failure with no fate to report — the
/// fate channel dying, which the run's close guard exists to prevent
/// — answers as HTTP does, with a status.
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

/// `POST /dequeue`: clear the running conversation's queue.
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

/// A refusal before the run exists: this run's queue closed, then the
/// status and the reason. The claim is the caller's local, and drops
/// on its return.
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
