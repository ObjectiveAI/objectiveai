//! The `claude_code` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `claude_code`, per the Container section of the
//! provider specification: one POST at `/` on port 8080 carries the
//! caller's request JSON in, MCP is asked on port 8081, Postgres
//! opened to port 8082. Beside the run, the queue's two verbs: `POST
//! /enqueue` and `POST /dequeue`, per the SDK's
//! `agentic_loop_container` module — the caller's way into the
//! conversation already running, backed by the [`spawn`] module's
//! writer task rather than a queue of its own: Claude Code holds the
//! queue, and this container holds the fates.

// The harness that consumes them comes later; the allows leave with it.
#[allow(dead_code)]
mod continuation;
#[allow(dead_code)]
mod response;
// Only `spawn`'s run itself is unconsumed — the queue verbs below are
// live; the allow leaves with the root handler.
#[allow(dead_code)]
mod spawn;
#[allow(dead_code)]
mod stdin;

use axum::Json;
use axum::http::StatusCode;
use diverge_provider_sdk::agentic_loop_container;

/// The loop port of the Container section of the provider
/// specification: where the server POSTs the request in.
const PORT: u16 = 8080;

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
        .route("/dequeue", axum::routing::post(dequeue));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// The run itself — not yet.
///
/// The root handling needs the StdoutMessage→chunk conversion, the
/// one-request claim, and the continuation read-back at exit; until
/// those land, the door answers honestly that it is not built. The
/// queue endpoints below are already real: an enqueue against a
/// container whose run never starts is missed, which is [`spawn`]'s
/// answer for a channel never opened.
async fn serve(
    Json(_request): Json<agentic_loop_container::request::Request>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "kind": "unimplemented",
            "error": "the claude_code run is not implemented yet",
        })),
    )
}

/// A message for the running conversation's queue.
///
/// The response IS the fate, and it arrives when the fate is known —
/// taken into the conversation (the replay echo says so), withdrawn
/// by a dequeue, or outlived by the run. That can be long after the
/// ask; nothing here times anything out. The one failure with no
/// fate to report — the fate channel dying, which the writer's own
/// close handling exists to prevent — answers as HTTP does, with a
/// status.
async fn enqueue(
    Json(request): Json<agentic_loop_container::enqueue::Request>,
) -> Result<
    Json<agentic_loop_container::enqueue::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    let fate = spawn::enqueue(request.prompt).await;
    match fate.await {
        // The writer speaks the SDK's own response type, so the fate
        // forwards as itself.
        Ok(response) => Ok(Json(response)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "kind": "fate_lost",
                "error": "the message's fate was never decided",
            })),
        )),
    }
}

/// Clear the running conversation's queue.
///
/// Naive, deliberately: whatever is pending is withdrawn — each
/// message's own `/enqueue` answers `dequeued` — and a queue with
/// nothing pending, closed, or never opened answers `empty`.
async fn dequeue(
    Json(_request): Json<agentic_loop_container::dequeue::Request>,
) -> Json<agentic_loop_container::dequeue::Response> {
    Json(spawn::dequeue().await)
}
