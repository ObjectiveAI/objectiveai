//! HTTP functions for function inventions.

use crate::{HttpClient, HttpError, McpHandler, Notifier};
use futures::Stream;

/// Creates a function invention (non-streaming).
pub async fn create_function_invention_unary(
    client: &HttpClient,
    mut params: super::request::FunctionInventionCreateParams,
) -> Result<super::response::unary::FunctionInvention, HttpError> {
    params.stream = None;
    client
        .send_unary(reqwest::Method::POST, "functions/inventions", Some(params))
        .await
}

/// Creates a streaming function invention. Returns
/// `(Stream<Chunk>, Notifier)`; see
/// [`crate::agent::completions::http::create_agent_completion_streaming`]
/// for the demux + handler semantics.
pub async fn create_function_invention_streaming<H: McpHandler>(
    client: &HttpClient,
    mut params: super::request::FunctionInventionCreateParams,
    handler: H,
) -> Result<
    (
        impl Stream<
            Item = Result<
                super::response::streaming::FunctionInventionChunk,
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
            "functions/inventions",
            params,
            handler,
        )
        .await
}
