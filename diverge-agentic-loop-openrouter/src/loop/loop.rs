//! The loop itself: connect, learn the tools, run the turns.

use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::openrouter;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;
use futures_util::Stream;
use rmcp::ServiceExt as _;
use rmcp::transport::StreamableHttpClientTransport;

use super::Error;
use crate::continuation::Continuation;
use crate::request::Tool;

/// Where the in-container MCP proxy serves: its hard-coded port —
/// the Container specification's MCP port — and rmcp's conventional
/// path, on loopback.
const MCP_PROXY: &str = "http://localhost:8081/mcp";

/// Run the whole agentic loop, and stream what it produces.
///
/// [`fetch`](crate::fetch::fetch) is one OpenRouter call; this is
/// that call in a loop, with the container's MCP proxy on the other
/// side of every tool call. The stream is the same shape as fetch's,
/// erring in the loop's own vocabulary — which includes fetch's,
/// wholesale.
pub async fn r#loop(
    api_key: &str,
    agent: openrouter::Agent,
    continuation: Option<Continuation>,
    prompt: Vec<rmcp::model::ContentBlock>,
) -> Result<
    impl Stream<Item = Result<AgenticLoopChunk, Error>> + Send + Unpin + use<>,
    Error,
> {
    // The MCP connection: the proxy beside us, dialled once per run.
    // The running service must outlive every tool call, so the loop
    // owns it for the life of the stream.
    let mcp = ()
        .serve(StreamableHttpClientTransport::from_uri(MCP_PROXY))
        .await?;

    // Every tool the caller's servers offer, through the proxy's one
    // listing — the paginating helper, so a cursor never truncates
    // the set.
    let tools: Vec<Tool> = mcp
        .list_all_tools()
        .await
        .map_err(Error::ListTools)?
        .into_iter()
        .map(Tool::new)
        .collect();

    let _ = (api_key, agent, continuation, prompt, tools);
    unimplemented!("the turns themselves");

    // An opaque return type must be inferable from somewhere, and a
    // panic constrains nothing; the anchor dies with the
    // implementation above it.
    #[allow(unreachable_code)]
    Ok(futures_util::stream::empty())
}
