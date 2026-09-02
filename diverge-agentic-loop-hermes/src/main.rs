//! The `hermes` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `hermes`, per the Container section of the
//! provider specification: a WebSocket at `/` on port 8080, the
//! server's first message the caller's request, every message back
//! one frame of the container response vocabulary — the loop's
//! chunks, the container's own `fetch_resource` asks for the agent's
//! `*_resource` identities (answered by the server on the
//! `/resource/{identity}` routes, into [`resource`]'s store), the
//! `fetch_continuation` ask every run opens with (answered on the
//! `/continuation` routes, into [`continuation_fetcher`]'s slot), and
//! the run's new continuation as the closer. Beside the run, the
//! queue's two verbs: `POST /enqueue` and `POST /dequeue`, per the
//! SDK's `agentic_loop_container` module — the caller's way into the
//! conversation already running.
//!
//! BOOTSTRAP: the run itself is not implemented yet, so the skeleton
//! serves the container's whole surface and refuses the run
//! honestly: the socket at `/` reads the request and answers with
//! one fatal `not_implemented` notification, the queue answers as a
//! container whose run will never begin (`missed` on enqueue,
//! `empty` on dequeue). The delivery routes are real already —
//! resources and the continuation land in their stores, where the
//! run will collect them.

// The continuation module is complete and unwired: the run driver
// that checks it before the gateway and streams it out after does
// not exist yet, and its dead-code warnings are the reminder.
mod continuation;
mod continuation_fetcher;
// The prepare module is complete and unwired for the same reason.
mod prepare;
mod resource;
mod resource_fetcher;
// The response module carries the gateway's COMPLETE run-event
// vocabulary. Until the run exists to read it, its dead-code
// warnings stand as the honest reminder of exactly that.
mod response;

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::http::StatusCode;
use diverge_provider_sdk::agentic_loop_container;
use diverge_provider_sdk::agentic_loop_container::response::Response;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::{
    AgenticLoopChunk, NotificationChunk,
};

/// The loop port of the Container section of the provider
/// specification: where the server opens the run's socket.
const PORT: u16 = 8080;

/// Whether the container's one run has arrived.
///
/// A container is one run: its queue, its MCP session, its
/// filesystem are all one conversation's, and a second run would
/// share all of them with the first. So the FIRST socket claims the
/// container for good — an atomic swap, so two arrivals a nanosecond
/// apart resolve to exactly one winner — and everything after it,
/// concurrent or later, is refused with `409` before the upgrade: a
/// first run that fails every later check has still spent the
/// container, because "one run" is a fact about arrivals, not about
/// merit.
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
        .route("/", axum::routing::get(serve))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue))
        .route("/continuation", axum::routing::post(continuation_chunk))
        .route(
            "/continuation/complete",
            axum::routing::post(continuation_complete),
        )
        .route(
            "/continuation/error",
            axum::routing::post(continuation_error),
        )
        .route(
            "/resource/{identity}",
            axum::routing::post(resource_chunk),
        )
        .route(
            "/resource/{identity}/complete",
            axum::routing::post(resource_complete),
        )
        .route(
            "/resource/{identity}/error",
            axum::routing::post(resource_error),
        );

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// One socket, one run — once the loop exists.
///
/// The claim is judged first, exactly as it will be in the real
/// container: a second run is the CALLER's error (`409`) even while
/// the first can only be refused. Then the upgrade, and the refusal
/// on the socket: the request is read — the surface is honored that
/// far — and answered with one fatal `not_implemented` notification,
/// after which the socket closes.
async fn serve(
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)>
{
    if CLAIMED.swap(true, Ordering::SeqCst) {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "claimed",
                "error": "this container serves one run, and it has already begun",
            })),
        ));
    }

    Ok(ws.on_upgrade(|mut socket| async move {
        // The request is read and, for now, not judged: the refusal
        // is the same whatever it said.
        let _ = socket.recv().await;
        let refusal = AgenticLoopChunk::Notification(NotificationChunk {
            r#type: Default::default(),
            is_fatal: true,
            message: serde_json::json!({
                "kind": "not_implemented",
                "error": "the hermes agentic loop is not implemented yet",
            }),
            meta: None,
        });
        let mut buffer = Vec::new();
        if Response::Chunk(refusal)
            .encode(&mut Writer::new(&mut buffer))
            .is_ok()
        {
            let _ = socket.send(Message::Binary(buffer.into())).await;
        }
        let _ = socket.send(Message::Close(None)).await;
    }))
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

/// One chunk of the continuation into the slot the run will read.
///
/// The body is the bytes verbatim, so nothing can be malformed — the
/// one refusal is a delivery after the settlement (`409`), because
/// anything after a settlement is somebody's bug.
async fn continuation_chunk(
    body: axum::body::Bytes,
) -> Result<
    Json<agentic_loop_container::continuation::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(continuation_fetcher::STORE.chunk(&body).await, "continuation")
}

/// The completion: every chunk is in — or none at all, the fresh
/// start — and the run may collect.
async fn continuation_complete(
    Json(_request): Json<agentic_loop_container::continuation::complete::Request>,
) -> Result<
    Json<agentic_loop_container::continuation::complete::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(continuation_fetcher::STORE.complete().await, "continuation")
}

/// The failure: the bytes can never come, and the waiting run learns
/// so in the server's own words.
async fn continuation_error(
    Json(request): Json<agentic_loop_container::continuation::error::Request>,
) -> Result<
    Json<agentic_loop_container::continuation::error::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(
        continuation_fetcher::STORE.error(request.error).await,
        "continuation",
    )
}

/// One chunk into the store the run will read.
///
/// The body is the bytes verbatim, so nothing can be malformed —
/// the one refusal is a delivery for a settled identity (`409`),
/// because anything after a settlement is somebody's bug. Whether
/// the delivery was ever asked for is not judged: the server posts
/// only in answer to the socket's asks, and an unasked delivery is
/// inert.
async fn resource_chunk(
    axum::extract::Path(identity): axum::extract::Path<String>,
    body: axum::body::Bytes,
) -> Result<
    Json<agentic_loop_container::resource::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(resource::STORE.chunk(&identity, &body), "resource")
}

/// The completion: every chunk is in, the run may collect.
async fn resource_complete(
    axum::extract::Path(identity): axum::extract::Path<String>,
    Json(_request): Json<agentic_loop_container::resource::complete::Request>,
) -> Result<
    Json<agentic_loop_container::resource::complete::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(resource::STORE.complete(&identity), "resource")
}

/// The failure: the bytes can never come, and the waiting fetch
/// completes with the server's error instead.
async fn resource_error(
    axum::extract::Path(identity): axum::extract::Path<String>,
    Json(request): Json<agentic_loop_container::resource::error::Request>,
) -> Result<
    Json<agentic_loop_container::resource::error::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(resource::STORE.error(&identity, request.error), "resource")
}

/// The delivery routes' one verdict: taken (`received`), or refused
/// because the delivery was already settled (`409`). One helper for
/// both deliveries: the continuation's response is an alias of the
/// resource's.
fn settled(
    taken: bool,
    what: &str,
) -> Result<
    Json<agentic_loop_container::resource::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    if taken {
        Ok(Json(agentic_loop_container::resource::Response {
            r#type: Default::default(),
        }))
    } else {
        Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "kind": "settled",
                "error": format!("the {what} is already settled"),
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
