//! HTTP functions for laboratory executions.

use crate::{HttpClient, HttpError};
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

/// Creates a streaming laboratory execution.
pub async fn create_laboratory_execution_streaming(
    client: &HttpClient,
    mut params: super::request::LaboratoryExecutionCreateParams,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::LaboratoryExecutionChunk,
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
        .send_streaming(reqwest::Method::POST, "laboratories/executions", Some(params))
        .await
}
