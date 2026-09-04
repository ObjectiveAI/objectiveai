//! The MCP server inside an agentic_loop container.
//!
//! To the agent beside it, a fully compliant MCP server on the
//! container's loopback at `/mcp`. To the provider outside, a
//! WebSocket listener at `/`: the server connects in — one connection
//! at a time — and every MCP exchange the agent asks for is carried
//! out over that socket as a channel, per the SDK's
//! [`mcp_proxy`](diverge_provider_sdk::mcp_proxy) wire, to be answered
//! by the caller's servers on the far side of the provider protocol.

mod handler;
mod notifications;
mod peers;
mod proxy;
mod queue;
mod ws;

use std::sync::Arc;

use axum::Json;
use axum::http::StatusCode;
use diverge_provider_sdk::agentic_loop_container;

use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

/// The MCP port of the Container section of the provider
/// specification: the one port this program listens on, serving both
/// paths.
const PORT: u16 = 14979;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(run());
}

async fn run() {
    let proxy = Arc::new(proxy::Proxy::new());
    let peers = Arc::new(peers::Peers::new());

    tokio::spawn(notifications::run(
        Arc::clone(&proxy),
        Arc::clone(&peers),
    ));

    let mcp = StreamableHttpService::new(
        {
            let proxy = Arc::clone(&proxy);
            let peers = Arc::clone(&peers);
            move || {
                Ok(handler::ProxyHandler {
                    proxy: Arc::clone(&proxy),
                    peers: Arc::clone(&peers),
                })
            }
        },
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    let app = axum::Router::new()
        .route("/", axum::routing::any(ws::accept))
        .route("/enqueue", axum::routing::post(enqueue))
        .route("/dequeue", axum::routing::post(dequeue))
        .nest_service("/mcp", mcp)
        .with_state(Arc::clone(&proxy));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}

/// A message for the agent's conversation, folded onto the next tool
/// response the proxy relays.
///
/// Container-INTERNAL: the harness beside this proxy calls it over
/// loopback and maps the answer onto the protocol's own fate
/// vocabulary. The answer arrives when the fate is known — the next
/// fold names the response it rode in (the SHA-256 of that
/// response's raw text, the harness's correlation key), a dequeue
/// says it was withdrawn — and nothing here times anything out. The
/// request body is the SDK's own enqueue request; the answer is the
/// proxy's, because `attached` is not a protocol fate.
///
/// A dead answer wire cannot happen — every pending message is
/// answered by a fold or a dequeue, and the queue lives as long as
/// the proxy — but if it somehow did, it answers as HTTP: `500`.
async fn enqueue(
    Json(request): Json<agentic_loop_container::enqueue::Request>,
) -> Result<
    Json<queue::Enqueued>,
    (StatusCode, Json<serde_json::Value>),
> {
    match queue::QUEUE.enqueue(request.prompt).await.await {
        Ok(answer) => Ok(Json(answer)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "kind": "fate_lost",
                "error": "the message's answer was never decided",
            })),
        )),
    }
}

/// Clear the queue: every pending message is answered dequeued, and
/// the reply says how many were.
async fn dequeue(
    Json(_request): Json<agentic_loop_container::dequeue::Request>,
) -> Json<serde_json::Value> {
    let cancelled = queue::QUEUE.dequeue().await;
    Json(serde_json::json!({ "cancelled": cancelled }))
}
