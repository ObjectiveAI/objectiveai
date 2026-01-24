//! HTTP client functions for vector completions.

use crate::{HttpClient, HttpError};
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
/// Sets `stream: Some(true)` and returns a stream of chunks that can be
/// accumulated into a complete response.
pub async fn create_vector_completion_streaming(
    client: &HttpClient,
    mut params: super::request::VectorCompletionCreateParams,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::VectorCompletionChunk,
            HttpError,
        >,
    >
    + Send
    + 'static
    + use<>,
    HttpError,
> {
    params.stream = Some(true);
    client
        .send_streaming(
            reqwest::Method::POST,
            "vector/completions",
            Some(params),
        )
        .await
}
