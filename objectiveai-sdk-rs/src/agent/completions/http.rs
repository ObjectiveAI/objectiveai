//! HTTP functions for agent completions.

use crate::{HttpClient, HttpError, McpHandler, Notifier};
use futures::Stream;

/// Creates a agent completion (non-streaming).
///
/// Sends a request to the agent completions endpoint and waits for the
/// complete response.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `params` - Agent completion parameters (stream flag will be set to false)
///
/// # Returns
///
/// The complete agent completion response.
pub async fn create_agent_completion_unary(
    client: &HttpClient,
    mut params: super::request::AgentCompletionCreateParams,
) -> Result<super::response::unary::AgentCompletion, HttpError> {
    params.stream = None;
    client
        .send_unary(reqwest::Method::POST, "agent/completions", Some(params))
        .await
}

/// Creates a streaming agent completion.
///
/// Opens a WebSocket to the agent completions endpoint, sends `params`
/// as the body frame, and returns a `(Stream, Notifier)` tuple:
///
/// - The `Stream` yields only `AgentCompletionChunk`s — the same shape
///   the previous SSE endpoint emitted.
/// - The `Notifier` provides `notify_list_changed(...)` for
///   forwarding upstream MCP `notifications/{tools,resources}/list_changed`
///   observations to the API while the completion is running.
///
/// `handler` is invoked for every inbound objectiveai-mcp
/// `server_request` (tools/list, tools/call) the API forwards from a
/// proxy upstream that dialed `/objectiveai-mcp/{session_id}`. Pass
/// [`crate::http::RejectHandler`] if the calling client doesn't host
/// objectiveai-mcp — agents that declare `client_objectiveai_mcp`
/// will then fall through to the next fallback agent server-side.
pub async fn create_agent_completion_streaming<H: McpHandler>(
    client: &HttpClient,
    mut params: super::request::AgentCompletionCreateParams,
    handler: H,
) -> Result<
    (
        impl Stream<
            Item = Result<
                super::response::streaming::AgentCompletionChunk,
                HttpError,
            >,
        >
        + Send
        + Unpin
        + 'static
        + use<H>,
        Notifier,
    ),
    HttpError,
> {
    params.stream = Some(true);
    client
        .send_streaming_ws(
            reqwest::Method::POST,
            "agent/completions",
            params,
            handler,
        )
        .await
}
