use crate::{HttpClient, HttpError, McpHandler, Notifier};
use futures::Stream;

pub async fn compute_profile_unary(
    client: &HttpClient,
    mut params: super::request::FunctionProfileComputationCreateParams,
) -> Result<super::response::unary::FunctionProfileComputation, HttpError> {
    params.stream = None;
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/profiles/compute",
            Some(params),
        )
        .await
}

/// Streaming profile computation. Returns `(Stream<Chunk>, Notifier)`;
/// see [`crate::agent::completions::http::create_agent_completion_streaming`]
/// for the demux + handler semantics.
pub async fn compute_profile_streaming<H: McpHandler>(
    client: &HttpClient,
    mut params: super::request::FunctionProfileComputationCreateParams,
    handler: H,
) -> Result<
    (
        impl Stream<
            Item = Result<
                super::response::streaming::FunctionProfileComputationChunk,
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
            "functions/profiles/compute",
            params,
            handler,
        )
        .await
}
