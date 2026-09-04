//! The `hermes` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `hermes`, per the Container section of the
//! provider specification: a WebSocket at `/` on port 14978, the
//! server's first message the caller's request, every message back
//! one frame of the container response vocabulary — the loop's
//! chunks, the container's own `fetch_resource` asks for the agent's
//! `*_resource` identities (answered by the server on the
//! `/resource/{identity}` routes, into [`store::resource`]), the
//! `fetch_continuation` ask every run opens with (answered on the
//! `/continuation` routes, into [`store::continuation`]), the
//! resources the run rewrote, and the run's new continuation as the
//! closer. Beside the run, the queue's two verbs: `POST /enqueue`
//! and `POST /dequeue`, per the SDK's `agentic_loop_container`
//! module — the caller's way into the conversation already running.
//!
//! The run itself is [`run::run`]: the filesystem laid down from the
//! request, `hermes gateway` spawned and driven over `/v1/runs`, the
//! way back up. This file is the socket around it — the request
//! read, the fetcher's asks and the run's items interleaved onto one
//! socket, each as its frame.

mod fetcher;
mod filesystem;
// The response module carries the gateway's COMPLETE run-event
// vocabulary, which is more than the conversion consumes — a field
// parsed and never read is the completeness, not dead code.
#[allow(dead_code)]
mod response;
mod run;
mod store;

use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use diverge_provider_sdk::agentic_loop_container;
use diverge_provider_sdk::agentic_loop_container::response::Response;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::Resource;
use futures_util::StreamExt as _;
use futures_util::stream;

use crate::fetcher::{Ask, Fetcher};
use crate::run::{Item, notification};

/// The loop port of the Container section of the provider
/// specification: where the server opens the run's socket.
const PORT: u16 = 14978;

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
    runtime.block_on(serve_forever());
}

async fn serve_forever() {
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

/// One socket, one run.
///
/// The upgrade is refused for the one thing knowable before the
/// request is read: a run after the first is the CALLER's error —
/// `409`, the container is [`CLAIMED`]. Everything after the upgrade
/// is the stream's: see [`drive`]. The socket closes when the drive
/// returns, whatever it returned for.
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
        drive(&mut socket).await;
        // The close is a frame of its own; the socket is gone when
        // the future is.
        let _ = socket.send(Message::Close(None)).await;
    }))
}

/// One of the two things that reach the socket.
enum Outbound {
    /// The fetcher asking the server for something.
    Ask(Ask),
    /// The run saying something.
    Item(Result<Item, run::Error>),
}

/// The run, on the socket: the request read, the run started, and
/// then everything that reaches the socket, interleaved as it comes
/// — the fetcher's asks (the server's to consume) and the run's
/// items (the client's to receive), each as its frame, the
/// continuation's pieces last.
///
/// Every failure past the upgrade has no status left to set and says
/// so as a fatal `notification` — the loop's own vocabulary — after
/// which the socket closes: the first message not being the request
/// frame, the CALLER's; anything the run could not begin from (the
/// wrong agent kind, a prompt it cannot speak, a filesystem that
/// would not lay down, a gateway that would not start), as the run's
/// one [`Err`](run::Error). Failures once the gateway is up ride the
/// stream as the run's own fatal notifications, and the run still
/// closes with the continuation.
///
/// The two sources are merged into one stream: the ask channel ends
/// when the fetcher — moved into the run — is dropped after the
/// filesystem is prepared, and the items end when the run does. So
/// while the run is still waiting on a delivery, the asks are what
/// the socket carries; after that, only items.
async fn drive(socket: &mut WebSocket) {
    // The request: the first binary message, the wire's own frame.
    let request = match socket.recv().await {
        Some(Ok(Message::Binary(bytes))) => {
            match agentic_loop_container::request::Request::decode(&bytes) {
                Ok(request) => request,
                Err(error) => {
                    fail(
                        socket,
                        serde_json::json!({
                            "kind": "request",
                            "error": error.to_string(),
                        }),
                    )
                    .await;
                    return;
                }
            }
        }
        _ => {
            fail(
                socket,
                serde_json::json!({
                    "kind": "request",
                    "error": "the first message was not the request frame",
                }),
            )
            .await;
            return;
        }
    };

    let (fetcher, asks) = Fetcher::new();
    let items = run::run(&request, fetcher).map(Outbound::Item);
    let asks = stream::unfold(asks, |mut asks| async move {
        asks.recv().await.map(|ask| (Outbound::Ask(ask), asks))
    });
    let mut outbound = Box::pin(stream::select(asks, items));

    while let Some(outbound) = outbound.next().await {
        let sent = match outbound {
            Outbound::Ask(Ask::Resource(ask)) => {
                send(socket, &Response::FetchResource(ask)).await
            }
            Outbound::Ask(Ask::Continuation) => {
                send(socket, &Response::FetchContinuation).await
            }
            Outbound::Item(Ok(Item::Chunk(chunk))) => {
                send(socket, &Response::Chunk(chunk)).await
            }
            Outbound::Item(Ok(Item::Resource { name, body })) => {
                send(socket, &Response::Resource(Resource { name, body: &body }))
                    .await
            }
            Outbound::Item(Ok(Item::Continuation(piece))) => {
                send(socket, &Response::Continuation(&piece)).await
            }
            Outbound::Item(Err(error)) => {
                fail(
                    socket,
                    serde_json::json!({
                        "kind": "run",
                        "error": error.to_string(),
                    }),
                )
                .await;
                return;
            }
        };
        if !sent {
            return;
        }
    }
}

/// Write one frame as one binary message. Whether it went — a frame
/// that will not encode, or a socket that is gone, both end the run,
/// and nothing here can tell those apart or needs to.
async fn send(socket: &mut WebSocket, frame: &Response<'_>) -> bool {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_err() {
        return false;
    }
    socket.send(Message::Binary(buffer.into())).await.is_ok()
}

/// The run cannot go on: say so as the fatal notification. The
/// socket closes when the drive returns.
async fn fail(socket: &mut WebSocket, message: serde_json::Value) {
    let _ = send(socket, &Response::Chunk(notification(message, true))).await;
}

/// A message for the running conversation's queue.
///
/// The response IS the fate, and it arrives when the fate is known —
/// folded onto a tool response, withdrawn by a dequeue, taken as the
/// next turn's prompt, or outlived by the run. That can be long
/// after the ask; nothing here times out. The one failure with no
/// fate to report — the fate channel dying, which the runner's close
/// guard exists to prevent — answers as HTTP does, with a status.
async fn enqueue(
    Json(request): Json<agentic_loop_container::enqueue::Request>,
) -> Result<
    Json<agentic_loop_container::enqueue::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    let fate = run::QUEUE.enqueue(request.prompt).await;
    match fate.await {
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
    settled(store::continuation::STORE.chunk(&body).await, "continuation")
}

/// The completion: every chunk is in — or none at all, the fresh
/// start — and the run may collect.
async fn continuation_complete(
    Json(_request): Json<agentic_loop_container::continuation::complete::Request>,
) -> Result<
    Json<agentic_loop_container::continuation::complete::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(store::continuation::STORE.complete().await, "continuation")
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
        store::continuation::STORE.error(request.error).await,
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
    settled(store::resource::STORE.chunk(&identity, &body), "resource")
}

/// The completion: every chunk is in, the run may collect.
async fn resource_complete(
    axum::extract::Path(identity): axum::extract::Path<String>,
    Json(_request): Json<agentic_loop_container::resource::complete::Request>,
) -> Result<
    Json<agentic_loop_container::resource::complete::Response>,
    (StatusCode, Json<serde_json::Value>),
> {
    settled(store::resource::STORE.complete(&identity), "resource")
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
    settled(store::resource::STORE.error(&identity, request.error), "resource")
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

/// Clear the running conversation's queue.
///
/// Whatever is pending is withdrawn — here and at the proxy — and
/// each message's own `/enqueue` answers `dequeued`; a queue with
/// nothing pending answers `empty`.
async fn dequeue(
    Json(_request): Json<agentic_loop_container::dequeue::Request>,
) -> Json<agentic_loop_container::dequeue::Response> {
    if run::QUEUE.dequeue().await {
        Json(agentic_loop_container::dequeue::Response::Dequeued {
            r#type: Default::default(),
        })
    } else {
        Json(agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        })
    }
}
