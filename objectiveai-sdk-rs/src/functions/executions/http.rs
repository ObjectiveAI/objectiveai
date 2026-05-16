//! HTTP functions for function executions.

use crate::{HttpClient, HttpError};
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

/// Creates a streaming function execution.
pub async fn create_function_execution_streaming(
    client: &HttpClient,
    mut params: super::request::FunctionExecutionCreateParams,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::FunctionExecutionChunk,
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
        .send_streaming(reqwest::Method::POST, "functions/executions", Some(params))
        .await
}
