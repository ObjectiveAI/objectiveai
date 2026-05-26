//! HTTP functions for function executions.

use crate::{HttpClient, HttpError, McpHandler, Notifier};
use futures::Stream;

/// Creates a function execution (non-streaming).
pub async fn create_function_execution_unary(
    client: &HttpClient,
    mut params: super::request::FunctionExecutionCreateParams,
) -> Result<super::response::unary::FunctionExecution, HttpError> {
    params.stream = None;
    client
        .send_unary(reqwest::Method::POST, "functions/executions", Some(params))
        .await
}

/// Creates a streaming function execution. Returns
/// `(Stream<Chunk>, Notifier)`; see
/// [`crate::agent::completions::http::create_agent_completion_streaming`]
/// for the demux + handler semantics.
pub async fn create_function_execution_streaming<H: McpHandler>(
    client: &HttpClient,
    mut params: super::request::FunctionExecutionCreateParams,
    handler: H,
) -> Result<
    (
        impl Stream<
            Item = Result<
                super::response::streaming::FunctionExecutionChunk,
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
            "functions/executions",
            params,
            handler,
        )
        .await
}
