//! HTTP client functions for the error endpoint.
use crate::{HttpClient, HttpError, McpHandler, Notifier};
use futures::Stream;

/// Creates an error response (unary).
pub async fn create_error_unary(
    client: &HttpClient,
    mut params: super::request::ErrorCreateParams,
) -> Result<super::response::ErrorResponse, HttpError> {
    params.stream = None;
    client
        .send_unary(reqwest::Method::POST, "error", Some(params))
        .await
}

/// Creates an error response with streaming. Returns
/// `(Stream<Chunk>, Notifier)`; see
/// [`crate::agent::completions::http::create_agent_completion_streaming`]
/// for the demux + handler semantics.
pub async fn create_error_streaming<H: McpHandler>(
    client: &HttpClient,
    mut params: super::request::ErrorCreateParams,
    handler: H,
) -> Result<
    (
        impl Stream<Item = Result<super::response::ErrorResponse, HttpError>>
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
        .send_streaming_ws(reqwest::Method::POST, "error", params, handler)
        .await
}
