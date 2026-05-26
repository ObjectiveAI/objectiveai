//! HTTP functions for laboratory executions.

use crate::{HttpClient, HttpError, McpHandler, Notifier};
use futures::Stream;

/// Creates a laboratory execution (non-streaming).
pub async fn create_laboratory_execution_unary(
    client: &HttpClient,
    mut params: super::request::LaboratoryExecutionCreateParams,
) -> Result<super::response::unary::LaboratoryExecution, HttpError> {
    params.stream = None;
    client
        .send_unary(reqwest::Method::POST, "laboratories/executions", Some(params))
        .await
}

/// Creates a streaming laboratory execution. Returns
/// `(Stream<Chunk>, Notifier)`; see
/// [`crate::agent::completions::http::create_agent_completion_streaming`]
/// for the demux + handler semantics.
pub async fn create_laboratory_execution_streaming<H: McpHandler>(
    client: &HttpClient,
    mut params: super::request::LaboratoryExecutionCreateParams,
    handler: H,
) -> Result<
    (
        impl Stream<
            Item = Result<
                super::response::streaming::LaboratoryExecutionChunk,
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
            "laboratories/executions",
            params,
            handler,
        )
        .await
}
