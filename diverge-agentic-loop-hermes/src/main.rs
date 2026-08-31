//! The `hermes` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `hermes`, per the Container section of the
//! provider specification: one POST at `/` on port 8080 carries the
//! caller's request JSON in, and the answer is a server-sent event
//! stream, each event one item of the container response vocabulary
//! — the loop's chunks, and the container's own `fetch_resource`
//! asks for the agent's `*_resource` identities, answered by the
//! server at `POST /resource`: chunks into [`resource`]'s store,
//! completion last. Beside the run, the queue's two verbs: `POST
//! /enqueue` and `POST /dequeue`, per the SDK's
//! `agentic_loop_container` module — the caller's way into the
//! conversation already running.
//!
//! BOOTSTRAP: the run itself is not implemented yet, so the
//! skeleton serves the container's whole surface and refuses the
//! run honestly: `/` answers `501`, and the queue answers as a
//! container whose run will never begin (`missed` on enqueue,
//! `empty` on dequeue). `/resource` is real already — deliveries
//! land in the store, where the run will collect them.

mod resource;

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_provider_sdk::agentic_loop_container;
use diverge_provider_sdk::decode::Decode as _;

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
    let app = axum::Router::new()
        .route("/", axum::routing::post(serve))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue))
        .route("/resource", axum::routing::post(resource_delivery));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// One request, one stream — once the loop exists.
///
/// The claim is judged first, exactly as it will be in the real
/// container: a second request is the CALLER's error (`409`) even
/// while the first can only be refused. Then the refusal: the Hermes
/// loop is not implemented, `501`. The `Sse` arm of the signature is
/// the shape the implementation will fill; no stream is ever built
/// here, so its type is the empty stream's, concretely.
async fn serve(
    Json(_request): Json<agentic_loop_container::request::Request>,
) -> Result<
    Sse<futures_util::stream::Empty<Result<Event, axum::Error>>>,
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

    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "kind": "not_implemented",
            "error": "the hermes agentic loop is not implemented yet",
        })),
    ))
}

/// A message for the conversation's queue.
///
/// No run will ever begin in this bootstrap, so every message meets
/// the fate of outliving nothing: `missed` — the SDK's word for a
/// message the conversation ended (here: never started) without
/// taking.
async fn enqueue(
    Json(_request): Json<agentic_loop_container::enqueue::Request>,
) -> Json<agentic_loop_container::enqueue::Response> {
    Json(agentic_loop_container::enqueue::Response::Missed {
        r#type: Default::default(),
    })
}

/// A resource delivery into the store the run will read.
///
/// Chunks append and the completion seals — [`resource::Store`] is
/// the whole of the behavior. A body that does not parse is its
/// sender's problem (`400`); a delivery for a sealed identity is
/// refused (`409`), because bytes after "whole" are somebody's bug.
/// Whether the delivery was ever asked for is not judged: the
/// server posts only in answer to the stream's asks, and an unasked
/// delivery is inert.
async fn resource_delivery(
    body: axum::body::Bytes,
) -> Result<
    Json<agentic_loop_container::resource::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    let request =
        match agentic_loop_container::resource::Request::decode(&body) {
            Ok(request) => request,
            Err(error) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "kind": "malformed",
                        "error": error.to_string(),
                    })),
                ));
            }
        };
    let taken = match request {
        agentic_loop_container::resource::Request::Chunk {
            identity,
            body,
        } => resource::STORE.chunk(&identity, body),
        agentic_loop_container::resource::Request::Complete { identity } => {
            resource::STORE.complete(&identity)
        }
    };
    if taken {
        Ok(Json(agentic_loop_container::resource::Response {
            r#type: Default::default(),
        }))
    } else {
        Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "complete",
                "error": "the resource is already complete",
            })),
        ))
    }
}

/// Clear the conversation's queue.
///
/// Nothing is ever held pending in this bootstrap, so the clearing
/// always finds nothing: `empty`.
async fn dequeue(
    Json(_request): Json<agentic_loop_container::dequeue::Request>,
) -> Json<agentic_loop_container::dequeue::Response> {
    Json(agentic_loop_container::dequeue::Response::Empty {
        r#type: Default::default(),
    })
}
