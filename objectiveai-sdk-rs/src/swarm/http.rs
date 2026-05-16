//! HTTP client functions for Swarm endpoints.

use crate::{HttpClient, HttpError};

/// Lists all Swarms that have been used.
pub async fn list_swarms(
    client: &HttpClient,
    params: super::request::ListSwarmsRequest,
) -> Result<super::response::ListSwarmResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "swarms/list", Some(params))
        .await
}

/// Retrieves a specific Swarm.
pub async fn get_swarm(
    client: &HttpClient,
    params: super::request::GetSwarmRequest,
) -> Result<super::response::GetSwarmResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "swarms", Some(params))
        .await
}

/// Retrieves usage statistics for a specific Swarm.
pub async fn get_swarm_usage(
    client: &HttpClient,
    params: super::request::GetSwarmRequest,
) -> Result<super::response::UsageSwarmResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "swarms/usage", Some(params))
        .await
}
