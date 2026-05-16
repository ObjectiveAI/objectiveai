//! HTTP functions for agent completions.

use crate::{HttpClient, HttpError};
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
/// Sends a request to the agent completions endpoint and returns a stream
/// of response chunks as they arrive via Server-Sent Events.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `params` - Agent completion parameters (stream flag will be set to true)
///
/// # Returns
///
/// A stream of agent completion chunks.
pub async fn create_agent_completion_streaming(
    client: &HttpClient,
    mut params: super::request::AgentCompletionCreateParams,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::AgentCompletionChunk,
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
        .send_streaming(reqwest::Method::POST, "agent/completions", Some(params))
        .await
}

/// Notifies a running agent completion with a user message.
///
/// Pushes a [`super::message::RichContent`] payload at the agent
/// completion identified by `params.response_id`; the api queues it
/// and surfaces it to the model on its next natural inspection point.
/// There is no response body — any 2xx status returns `Ok(())`.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `params` - The notify parameters (`response_id` + `content`)
pub async fn notify_agent_completion(
    client: &HttpClient,
    params: super::request::AgentCompletionNotifyParams,
) -> Result<(), HttpError> {
    client
        .send_unary_no_response(
            reqwest::Method::POST,
            "agent/completions/notify",
            Some(params),
        )
        .await
}
