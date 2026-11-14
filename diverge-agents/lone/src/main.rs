//! The `lone` agent, as a container.
//!
//! The program an agent container runs for a LoNE agent: an HTTP
//! server on the container's loopback, at the port the SDK's
//! [`diverge_sdk::container_proxy::inside::port()`] names, that the proxy beside it
//! forwards the provider's asks to. `POST /run` runs a loop — one at
//! a time, the lock held for exactly the stream's life, so a run
//! after the first resumes the conversation and a run beside it is
//! refused — and streams its chunks back as server-sent events;
//! `POST /enqueue` is the running loop's queue, answered at once;
//! `--schema`, the program's one argument, prints the JSON Schema of
//! the agent value this image accepts (see [`main`]); `POST
//! /register` is that value, told once for the container's life
//! before any loop — a run before it is refused — and is when its
//! source is checked: exactly one of `bundle` and `endpoint`, and a
//! bundle's manifest `lone convert` writes; answered
//! with the secrets this image takes, which are none; `POST /secrets`
//! is their values, fed by the proxy once the registration is
//! answered and again at every activation, held and read by nothing;
//! `POST /secrets/collect` is the rotating secrets asked back at
//! every inactive, which this image, declaring none, is never asked.
//! No upstream model: every turn RUNS `lone harness` against the whole
//! conversation and the caller's tool and resource lists, and reads
//! the chunks it writes. Its tool calls go through the
//! proxy's MCP server; the history it resumes from, and leaves
//! behind, is one row in the caller's database, reached through the
//! proxy's loopback pgwire.
//!
//! # Nothing of the proxy's before a request
//!
//! The proxy is not part of this image: the host injects it at
//! runtime, and it may not be up at all until a request comes. So
//! the server binds and waits, and the first thing to touch the
//! proxy — the database, the MCP session — is `POST /run` itself.
//! Registration touches only a bundle's directory, never the proxy.
//! Making the [`Client`] is no I/O.
//!
//! # What is an error, and what is not
//!
//! Anything that fails before the loop has said a single thing is a
//! real error — a non-`2xx`, with a JSON reason, and no stream:
//! arguments that are not a LoNE agent, a source that is not exactly
//! one of `bundle` and `endpoint`, a bundle that is not there, a
//! database that will not answer, a history that will not open, an
//! MCP session that will not connect — and the first
//! item of the loop itself, which is pulled before the response is
//! decided. That first item is the run's messages' user parts, said
//! before the harness runs: `/run` answers once the messages are
//! taken, never after turn one. A failure AFTER output — the first
//! run of the harness among them: raising, breaking the harness,
//! answering the wrong shape, saying a chunk it may not, calling a
//! tool that does not exist — is a fatal `notification` chunk, the
//! loop's own last word, then the stream's end. The stream never
//! carries an error of its own: it is chunks, and only chunks.

mod agent;
mod claim;
mod continuation;
mod history;
mod r#loop;
mod queue;
mod registration;
mod run;
mod secrets;

use std::convert::Infallible;
use std::io::Write as _;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::container_proxy::inside::agent::enqueue;
use diverge_sdk::container_proxy::inside::agent::run as run_endpoint;
use diverge_sdk::container_proxy::inside::register;
use diverge_sdk::container_proxy::inside::register::response::Response;
use diverge_sdk::container_proxy::inside::secrets::{collect, feed};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::{
    AgenticLoopChunk, NotificationChunk,
};
use futures_util::{Stream, StreamExt as _};
use serde_json::Value;

use crate::agent::Agent;
use crate::claim::Claim;
use crate::continuation::Continuation;
use crate::r#loop::Item;
use crate::queue::{Closed, QUEUE};

/// A refusal: the status, and a JSON reason.
type Refusal = (StatusCode, Json<Value>);

/// The program: the server, or — on `--schema` — the schema.
///
/// The program takes one argument, `--schema`, and no other: exactly
/// that prints the JSON Schema of [`Agent`] on stdout and exits 0
/// ([`schema`]); no argument is the server, for the container's
/// life; anything else — another argument, or more than one — is a
/// usage line on stderr and exit 2. The image's ENTRYPOINT never
/// passes the argument. It exists so the image's `/DIVERGE.md`,
/// whose frontmatter's `schema` is this schema and which the proxy reads for
/// what the `arguments` may be, is generated from the type the run
/// reads — `cargo run -- --schema`, by the assembler — and never
/// written by hand, so the file and the type cannot disagree.
fn main() {
    let mut arguments = std::env::args_os().skip(1);
    match (arguments.next(), arguments.next()) {
        (None, None) => {}
        (Some(argument), None) if argument == "--schema" => return schema(),
        _ => {
            usage();
            std::process::exit(2);
        }
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(serve());
}

/// `--schema`: the JSON Schema of [`Agent`], pretty-printed on stdout
/// with a trailing newline.
///
/// This is the one thing the program ever writes to stdout, and
/// deliberately: stdout IS the answer, read whole by the assembler
/// that writes `/DIVERGE.md`. Either failure is said on stderr and
/// the exit is 1 — never a partial or empty schema passed off as
/// the answer: serialising, which cannot fail (a [`schemars::Schema`]
/// is a JSON value already), and the write, which can, when the
/// reader closes the pipe early.
fn schema() {
    let text = match serde_json::to_string_pretty(&schemars::schema_for!(Agent)) {
        Ok(text) => text,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = writeln!(std::io::stdout(), "{text}") {
        let _ = writeln!(std::io::stderr(), "{error}");
        std::process::exit(1);
    }
}

/// The usage line, on stderr, for arguments that are not the one.
fn usage() {
    let _ = writeln!(
        std::io::stderr(),
        "usage: diverge-agentic-loop-lone [--schema]"
    );
}

/// The server, for the container's life.
///
/// Bound on the loopback: the proxy is the only thing that dials it,
/// and the proxy is beside it.
async fn serve() {
    let app = axum::Router::new()
        .route("/register", axum::routing::post(register))
        .route("/run", axum::routing::post(run))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/secrets", axum::routing::post(feed))
        .route("/secrets/collect", axum::routing::post(collect))
        .with_state(Arc::new(Client::new()));

    let listener =
        tokio::net::TcpListener::bind(("127.0.0.1", diverge_sdk::container_proxy::inside::port()))
            .await
            .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// `POST /run`: the loop.
///
/// Prepares everything the loop needs, starts it, and pulls its FIRST
/// item before answering — the messages' user parts, said before the
/// harness runs, so the answer comes once the messages are taken: an
/// error there is a real error, the status and the reason, with no
/// stream begun. Then the stream — the first
/// chunk, then every chunk as it comes, each history at rest saved as
/// it is reached, and a failure after the first chunk a fatal
/// notification.
///
/// A run before registration is refused (`409`) before anything
/// else. One run at a time: the [`Claim`] is taken next and refused
/// with `409` while another holds it, and it lives exactly as long
/// as the stream — released on every refusal below, and otherwise
/// captured into the stream, so it drops with it, finished or
/// abandoned. The queue is opened for this run right behind the
/// claim; every refusal before the loop exists closes it again, so an
/// `/enqueue` meant for a loop that will never look is refused `409`
/// `closed`, and the proxy keeps the batch for the next `/run`. Once
/// the loop exists its own guard closes the queue however the run
/// ends. The history comes from the cache when this container has
/// loaded or saved one, and from the row only the first time.
async fn run(
    State(client): State<Arc<Client>>,
    Json(request): Json<run_endpoint::request::Request>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Refusal> {
    // The run is a chat run, or it is refused: this agent's image
    // declares no structured decisions, so a decision is never taken.
    // Refused before anything is claimed, so nothing is to undo.
    let request = match request {
        run_endpoint::request::Request::Chat { messages } => enqueue::request::Request { messages },
        run_endpoint::request::Request::StructuredDecisions(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "kind": "content",
                    "error": "structured decisions are not supported by this agent",
                })),
            ));
        }
    };
    if !registration::registered() {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "unregistered",
                "error": "no agent has been registered",
            })),
        ));
    }
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
                return Err(refuse(
                    generation,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error.message(),
                )
                .await);
            }
        },
    };

    if request.messages.is_empty()
        || request
            .messages
            .iter()
            .any(|message| message.content.is_empty())
    {
        return Err(refuse(
            generation,
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "kind": "content",
                "error": "a message needs content",
            }),
        )
        .await);
    }

    let mut items = match r#loop::r#loop(&client, continuation, request.messages, generation).await
    {
        Ok(items) => items,
        Err(error) => {
            return Err(refuse(
                generation,
                StatusCode::INTERNAL_SERVER_ERROR,
                error.message(),
            )
            .await);
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

/// `POST /register`: the `arguments`, the agent, once, for the
/// container's life — checked on the way in.
///
/// A value this image will not take is `400`, and so is a `bundle`
/// with no `manifest.json`; an agent already registered is `409`,
/// whatever the second carries — the agent never changes. `200`, with
/// the tools the agent depends on and the secrets it takes — none —
/// passed back, is the agent held and its bundle checked.
async fn register(
    Json(request): Json<register::request::Request>,
) -> Result<(StatusCode, Json<Response>), Refusal> {
    let agent: Agent = match serde_json::from_value(request.arguments) {
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
    let dependencies = agent.dependencies.clone();
    let secrets = agent.declaration();
    match registration::register(agent).await {
        Ok(()) => Ok((
            StatusCode::OK,
            Json(Response {
                dependencies,
                secrets,
                permissions: Vec::new(),
            }),
        )),
        Err(error) => Err((error.status(), Json(error.message()))),
    }
}

/// `POST /secrets`: the secrets' values, fed.
///
/// Held whole, replacing whatever the last feed left, and read by no
/// loop — this image declares no secret, so a feed carries nothing
/// it wants. `204`, since the proxy reads no body: the answer is the
/// values held, and nothing is delivered to this program until it is
/// given.
async fn feed(Json(request): Json<feed::request::Request>) -> StatusCode {
    secrets::feed(request.0).await;
    StatusCode::NO_CONTENT
}

/// `POST /secrets/collect`: the rotating secrets, handed back.
///
/// This image declares none, so the proxy never asks; served as the
/// contract all the same: every name asked, with the value it was
/// fed, and a name the program holds no value for `500`, naming the
/// name and never a value.
async fn collect(
    Json(request): Json<collect::request::Request>,
) -> Result<Json<collect::response::Response>, Refusal> {
    match secrets::collect(&request.names).await {
        Ok(values) => Ok(Json(collect::response::Response(values))),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "kind": "secrets",
                "error": error,
            })),
        )),
    }
}

/// `POST /enqueue`: the proxy's batch, onto the running loop's
/// queue, answered at once.
///
/// Content the program will not take is `400`, checked before
/// anything is accepted. Otherwise the batch joins whatever already
/// waits, and the answer is `204`: the messages are the loop's,
/// taken at its next seam as ONE user turn — with anything else
/// waiting — each message's user parts under its own key, and the
/// `204` is their delivery. Nothing waits on the loop, and nothing
/// withdraws a message once accepted. A run whose last look has
/// closed the queue — or no run at all — is `409`
/// `{"kind":"closed"}`, nothing accepted: the proxy keeps the batch
/// and starts the next run on it.
async fn enqueue(Json(request): Json<enqueue::request::Request>) -> Result<StatusCode, Refusal> {
    if request.messages.is_empty()
        || request
            .messages
            .iter()
            .any(|message| message.content.is_empty())
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "kind": "content",
                "error": "a message needs content",
            })),
        ));
    }
    match QUEUE.enqueue(request.messages).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(Closed) => Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "closed",
                "error": "the run has closed",
            })),
        )),
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
