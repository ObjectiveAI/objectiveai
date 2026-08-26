//! The `openrouter` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `openrouter`, per the Container section of the
//! provider specification: one POST at `/` on port 8080 carries the
//! caller's request JSON in, and the answer is a server-sent event
//! stream, each event one chunk of the response vocabulary. The
//! agent's tool calls go out as an MCP client against the in-container
//! proxy on port 8081.

// The continuation, API types and fetch land before the loop that
// will speak them; the allows leave with that wiring.
#[allow(dead_code)]
mod continuation;
#[allow(dead_code)]
mod error;
#[allow(dead_code)]
mod fetch;
#[allow(dead_code, unused_imports)]
mod request;
#[allow(dead_code, unused_imports)]
mod response;
mod serde_util;
#[allow(dead_code)]
mod stream_once;

use std::pin::Pin;

use axum::Json;
use axum::response::sse::{Event, Sse};
use diverge_provider_sdk::endpoints::agentic_loop::run::client;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;
use futures_util::{Stream, StreamExt as _};

/// The loop port of the Container section of the provider
/// specification: where the server POSTs the request in.
const PORT: u16 = 8080;

/// What the loop produces: the chunks, in the order they happen.
type ChunkStream = Pin<Box<dyn Stream<Item = AgenticLoopChunk> + Send>>;

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

/// One request, one stream: the caller's JSON arrives as the SDK's
/// own request type — the tag byte is the wire's, not HTTP's, so the
/// body is the bare object — and every chunk the loop produces leaves
/// as one SSE event carrying that chunk's JSON.
async fn serve(
    Json(request): Json<client::request::Frame>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    Sse::new(chunks(request).map(|chunk| Event::default().json_data(&chunk)))
}

/// The loop itself: everything between a request and its chunks.
fn chunks(request: client::request::Frame) -> ChunkStream {
    let _ = request;
    unimplemented!("the OpenRouter loop is not yet written")
}
