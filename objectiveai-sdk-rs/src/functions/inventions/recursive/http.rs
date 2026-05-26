//! HTTP functions for recursive function inventions.

use crate::{HttpClient, HttpError, McpHandler, Notifier};
use futures::Stream;

/// Creates a recursive function invention (non-streaming).
pub async fn create_function_invention_recursive_unary(
    client: &HttpClient,
    mut params: super::request::FunctionInventionRecursiveCreateParams,
) -> Result<super::response::unary::FunctionInventionRecursive, HttpError> {
    params.stream = None;
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/inventions/recursive",
            Some(params),
        )
        .await
}

/// Creates a streaming recursive function invention. Returns
/// `(Stream<Chunk>, Notifier)`; see
/// [`crate::agent::completions::http::create_agent_completion_streaming`]
/// for the demux + handler semantics.
pub async fn create_function_invention_recursive_streaming<H: McpHandler>(
    client: &HttpClient,
    mut params: super::request::FunctionInventionRecursiveCreateParams,
    handler: H,
) -> Result<
    (
        impl Stream<
            Item = Result<
                super::response::streaming::FunctionInventionRecursiveChunk,
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
            "functions/inventions/recursive",
            params,
            handler,
        )
        .await
}
