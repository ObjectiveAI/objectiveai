//! HTTP functions for function inventions.

use crate::{HttpClient, HttpError};
use futures::Stream;

/// Creates a function invention (non-streaming).
pub async fn create_function_invention_unary(
    client: &HttpClient,
    mut params: super::request::FunctionInventionCreateParams,
) -> Result<super::response::unary::FunctionInvention, HttpError> {
    params.stream = None;
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/inventions",
            Some(params),
        )
        .await
}

/// Creates a streaming function invention.
pub async fn create_function_invention_streaming(
    client: &HttpClient,
    mut params: super::request::FunctionInventionCreateParams,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::FunctionInventionChunk,
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
            "functions/inventions",
            Some(params),
        )
        .await
}
