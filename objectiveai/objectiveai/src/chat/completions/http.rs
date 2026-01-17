//! HTTP functions for chat completions.

use crate::{HttpClient, HttpError};
use futures::Stream;

/// Creates a chat completion (non-streaming).
///
/// Sends a request to the chat completions endpoint and waits for the
/// complete response.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `params` - Chat completion parameters (stream flag will be set to false)
///
/// # Returns
///
/// The complete chat completion response.
pub async fn create_chat_completion_unary(
    client: &HttpClient,
    mut params: super::request::ChatCompletionCreateParams,
) -> Result<super::response::unary::ChatCompletion, HttpError> {
    params.stream = None;
    client
        .send_unary(reqwest::Method::POST, "chat/completions", Some(params))
        .await
}

/// Creates a streaming chat completion.
///
/// Sends a request to the chat completions endpoint and returns a stream
/// of response chunks as they arrive via Server-Sent Events.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `params` - Chat completion parameters (stream flag will be set to true)
///
/// # Returns
///
/// A stream of chat completion chunks.
pub async fn create_chat_completion_streaming(
    client: &HttpClient,
    mut params: super::request::ChatCompletionCreateParams,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::ChatCompletionChunk,
            HttpError,
        >,
    >,
    HttpError,
> {
    params.stream = Some(true);
    client
        .send_streaming(reqwest::Method::POST, "chat/completions", Some(params))
        .await
}
