//! HTTP client functions for vector completions.

use crate::{HttpClient, HttpError, McpHandler, Notifier};
use futures::Stream;

/// Creates a vector completion and waits for the complete response.
///
/// Sets `stream: None` to ensure a unary response.
pub async fn create_vector_completion_unary(
    client: &HttpClient,
    mut params: super::request::VectorCompletionCreateParams,
) -> Result<super::response::unary::VectorCompletion, HttpError> {
    params.stream = None;
    client
        .send_unary(reqwest::Method::POST, "vector/completions", Some(params))
        .await
}

/// Creates a vector completion with streaming response.
///
/// Opens a WebSocket and returns `(Stream<Chunk>, Notifier)`. See
/// [`crate::agent::completions::http::create_agent_completion_streaming`]
/// for the full semantics of `handler` and the demux.
pub async fn create_vector_completion_streaming<H: McpHandler>(
    client: &HttpClient,
    mut params: super::request::VectorCompletionCreateParams,
    handler: H,
) -> Result<
    (
        impl Stream<
            Item = Result<
                super::response::streaming::VectorCompletionChunk,
                HttpError,
            >,
        > + Send
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
            "vector/completions",
            params,
            handler,
        )
        .await
}
