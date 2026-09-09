//! The `eliza` agent, as a container.
//!
//! The program an agent container runs for an Eliza (elizaOS) agent.
//! Its shape is the other containers': an HTTP server the proxy
//! beside it forwards the provider's asks to — `POST /run`, `GET
//! /schema`, `POST /enqueue`, `POST /dequeue` — with the agent value
//! this crate's own ([`agent`]) and its schema derived from it. The
//! design is `reports/4.md`.
//!
//! BOOTSTRAP: the run itself is not built yet. The agent is defined
//! and its schema served; `/run` refuses honestly with `501`, and the
//! queue answers as a container whose run will never begin (`missed`
//! on enqueue, `empty` on dequeue). The port and the one-run claim
//! are the old surface's, and go with the server chunk.

mod agent;

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_provider_sdk::container_proxy::agent::dequeue::Outcome;
use diverge_provider_sdk::container_proxy::agent::enqueue::Fate;
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
/// The claim is judged first; then the refusal: the Eliza loop is
/// not built, `501`. The `Sse` arm of the signature is the shape the
/// implementation will fill; no stream is ever built here, so its
/// type is the empty stream's, concretely.
async fn run(
    Json(_request): Json<run_loop::request::Request>,
) -> Result<
    Sse<futures_util::stream::Empty<Result<Event, axum::Error>>>,
    (StatusCode, Json<serde_json::Value>),
> {
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
