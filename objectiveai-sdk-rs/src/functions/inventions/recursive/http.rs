//! HTTP functions for recursive function inventions.

use crate::{HttpClient, HttpError};
use futures::Stream;

/// Creates a recursive function invention (non-streaming).
pub async fn create_function_invention_recursive_unary(
    client: &HttpClient,
    mut params: super::request::FunctionInventionRecursiveCreateParams,
) -> Result<super::response::unary::FunctionInventionRecursive, HttpError> {
    params.stream = None;
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/inventions/recursive",
            Some(params),
        )
        .await
}

/// Creates a streaming recursive function invention.
pub async fn create_function_invention_recursive_streaming(
    client: &HttpClient,
    mut params: super::request::FunctionInventionRecursiveCreateParams,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::FunctionInventionRecursiveChunk,
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
            "functions/inventions/recursive",
            Some(params),
        )
        .await
}
