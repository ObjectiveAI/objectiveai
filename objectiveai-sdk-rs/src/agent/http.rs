//! HTTP client functions for Agent endpoints.

use crate::{HttpClient, HttpError};

/// Lists all Agents that have been used.
pub async fn list_agents(
    client: &HttpClient,
    params: super::request::ListAgentsRequest,
) -> Result<super::response::ListAgentResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "agents/list", Some(params))
        .await
}

/// Retrieves a specific Agent.
pub async fn get_agent(
    client: &HttpClient,
    params: super::request::GetAgentRequest,
) -> Result<super::response::GetAgentResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "agents", Some(params))
        .await
}

/// Retrieves usage statistics for a specific Agent.
pub async fn get_agent_usage(
    client: &HttpClient,
    params: super::request::GetAgentRequest,
) -> Result<super::response::UsageAgentResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "agents/usage", Some(params))
        .await
}
