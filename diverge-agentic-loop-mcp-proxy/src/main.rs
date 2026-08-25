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
mod ws;

use std::sync::Arc;

use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

/// The MCP port of the Container section of the provider
/// specification: the one port this program listens on, serving both
/// paths.
const PORT: u16 = 8081;

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
        .nest_service("/mcp", mcp)
        .with_state(Arc::clone(&proxy));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", PORT))
        .await
        .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}
