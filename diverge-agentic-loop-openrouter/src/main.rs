//! The `openrouter` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `openrouter`, per the Container section of the
//! provider specification: one POST at `/` on port 8080 carries the
//! caller's request JSON in, and the answer is a server-sent event
//! stream, each event one chunk of the response vocabulary — the
//! turns behind it run by [`r#loop`](r#loop::r#loop). The agent's
//! tool calls go out as an MCP client against the in-container proxy
//! on port 8081.

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

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use diverge_provider_sdk::agentic_loop_container;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::Agent;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::NotificationChunk;
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
    let app = axum::Router::new().route("/", axum::routing::post(serve));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// One request, one stream.
///
/// The caller's JSON arrives as the SDK's own request type — the tag
/// byte is the wire's, not HTTP's, so the body is the bare object —
/// and every chunk the loop produces leaves as one SSE event carrying
/// that chunk's JSON.
///
/// The failures divide by whose they are:
///
/// - A request after the first is the CALLER's error, and the first
///   error checked: `409`, the container is [`CLAIMED`].
/// - An agent of another kind, or a continuation token that will not
///   open, is the CALLER's error: `400`.
/// - A missing `OPENROUTER_API_KEY` is the server's own
///   misconfiguration: `500`. The caller's request carries no
///   credential for the upstream; whose key the container runs with
///   is the image's business.
/// - The loop failing to start inherits OpenRouter's own verdict
///   where there is one — [`Error::status`](r#loop::Error::status) —
///   and answers `500` for the server's own machinery.
/// - An error after the stream began cannot change the status that
///   already left; it arrives IN the stream, as a `notification`
///   chunk with `is_fatal` set, and is the stream's last word.
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

    let Agent::Openrouter(agent) = request.agent else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "kind": "wrong_agent",
                "error": "this container serves the openrouter agent kind",
            })),
        ));
    };

    let api_key = match std::env::var("OPENROUTER_API_KEY") {
        Ok(api_key) => api_key,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "kind": "missing_api_key",
                    "error": "OPENROUTER_API_KEY is not set",
                })),
            ));
        }
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

    let chunks =
        match r#loop::r#loop(&api_key, agent, continuation, request.prompt)
            .await
        {
            Ok(chunks) => chunks,
            Err(error) => {
                return Err((
                    StatusCode::from_u16(error.status().as_u16())
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    Json(error.message()),
                ));
            }
        };

    Ok(Sse::new(chunks.map(|chunk| {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            // The stream has already begun; there is no status left to
            // change. The failure travels IN the stream, fatally, and
            // fetch ends the stream right after it.
            Err(error) => agentic_loop_container::response::Response::Notification(
                NotificationChunk {
                    r#type: Default::default(),
                    is_fatal: true,
                    message: error.message(),
                    meta: None,
                },
            ),
        };
        Event::default().json_data(&chunk)
    })))
}
