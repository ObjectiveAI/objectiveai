//! The `eliza` agent, as a container.
//!
//! The program an agent container runs for an Eliza (elizaOS) agent.
//! Its shape is the other containers': an HTTP server the proxy
//! beside it forwards the provider's asks to — `POST /register`,
//! `POST /run`, `GET /schema`, `POST /enqueue`, `POST /dequeue` — with the agent value
//! this crate's own ([`agent`]) and its schema derived from it. The
//! design is `reports/4.md`.
//!
//! BOOTSTRAP: the run itself is not built yet. The agent is defined,
//! its schema served and its registration taken; `/run` refuses
//! honestly with `501` once registered (and `409` before), and the
//! queue answers as a container whose run will never begin (`missed`
//! on enqueue, `empty` on dequeue). The port and the one-run claim
//! are the old surface's, and go with the server chunk.

mod agent;
mod registration;

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_provider_sdk::container_proxy::agent::dequeue::Outcome;
use diverge_provider_sdk::container_proxy::agent::enqueue::Fate;
use diverge_provider_sdk::container_proxy::register;
use diverge_provider_sdk::container_proxy::run_loop;
use diverge_provider_sdk::shared::containers::enqueue;

use crate::agent::Agent;

/// The old loop port; the server chunk replaces it with the SDK's
/// [`agent::port()`](diverge_provider_sdk::container_proxy::agent::port).
const PORT: u16 = 14978;

/// Whether the container's one request has arrived. The old one-run
/// claim; the server chunk replaces it with the three-phase claim
/// the other containers hold.
static CLAIMED: AtomicBool = AtomicBool::new(false);

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(serve());
}

async fn serve() {
    let app = axum::Router::new()
        .route("/register", axum::routing::post(register))
        .route("/run", axum::routing::post(run))
        .route("/schema", axum::routing::get(schema))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// `POST /run`: one request, one stream — once the loop exists.
///
/// Registration is judged first (`409` before it), then the claim;
/// then the refusal: the Eliza loop is not built, `501`. The `Sse` arm of the signature is the shape the
/// implementation will fill; no stream is ever built here, so its
/// type is the empty stream's, concretely.
async fn run(
    Json(_request): Json<run_loop::request::Request>,
) -> Result<
    Sse<futures_util::stream::Empty<Result<Event, axum::Error>>>,
    (StatusCode, Json<serde_json::Value>),
> {
    if registration::registered().is_none() {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "unregistered",
                "error": "no agent has been registered",
            })),
        ));
    }
    if CLAIMED.swap(true, Ordering::SeqCst) {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "busy",
                "error": "a run is in progress",
            })),
        ));
    }

    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "kind": "not_implemented",
            "error": "the eliza loop is not built yet",
        })),
    ))
}

/// `POST /register`: the agent, once, for the container's life.
///
/// A value this image will not take is `400`; an agent already
/// registered is `409`, whatever the second carries — the agent
/// never changes. `204` is the agent held.
async fn register(Json(request): Json<register::request::Request>) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
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
/// [`Agent`], derived from the type the run will read, so the two
/// cannot disagree.
async fn schema() -> Json<schemars::Schema> {
    Json(schemars::schema_for!(Agent))
}

/// `POST /enqueue`: a message for the conversation's queue.
///
/// No run will ever begin in this bootstrap, so every message meets
/// the fate of outliving nothing: `missed`.
async fn enqueue(Json(_request): Json<enqueue::request::Request>) -> Json<Fate> {
    Json(Fate::Missed)
}

/// `POST /dequeue`: clear the conversation's queue.
///
/// Nothing is ever held pending in this bootstrap, so the clearing
/// always finds nothing: `empty`.
async fn dequeue() -> Json<Outcome> {
    Json(Outcome::Empty)
}
