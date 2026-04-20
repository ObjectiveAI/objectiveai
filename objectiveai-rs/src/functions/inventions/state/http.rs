//! HTTP functions for function invention state retrieval.

use crate::{HttpClient, HttpError};

/// Retrieves a function invention state definition.
pub async fn get_function_invention_state(
    client: &HttpClient,
    params: crate::RemotePathCommitOptional,
) -> Result<super::response::GetFunctionInventionStateResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "functions/inventions/state", Some(params))
        .await
}
