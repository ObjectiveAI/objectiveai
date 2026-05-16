//! HTTP client functions for the error endpoint.
use crate::{HttpClient, HttpError};
use futures::Stream;

/// Creates an error response (unary).
pub async fn create_error_unary(
    client: &HttpClient,
    mut params: super::request::ErrorCreateParams,
) -> Result<super::response::ErrorResponse, HttpError> {
    params.stream = None;
    client
        .send_unary(reqwest::Method::POST, "error", Some(params))
        .await
}

/// Creates an error response with streaming (SSE).
pub async fn create_error_streaming(
    client: &HttpClient,
    mut params: super::request::ErrorCreateParams,
) -> Result<
    impl Stream<Item = Result<super::response::ErrorResponse, HttpError>> + Send + 'static + use<>,
    HttpError,
> {
    params.stream = Some(true);
    client
        .send_streaming(reqwest::Method::POST, "error", Some(params))
        .await
}
