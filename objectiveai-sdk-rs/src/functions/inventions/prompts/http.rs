//! HTTP functions for invention prompt management.

use crate::{HttpClient, HttpError};

/// Lists all prompts accessible to the authenticated user.
pub async fn list_prompts(
    client: &HttpClient,
    params: super::request::ListPromptsRequest,
) -> Result<super::response::ListPromptResponse, HttpError> {
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/inventions/prompts/list",
            Some(params),
        )
        .await
}

/// Retrieves a prompt definition.
pub async fn get_prompt(
    client: &HttpClient,
    params: super::request::GetPromptRequest,
) -> Result<super::response::GetPromptResponse, HttpError> {
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/inventions/prompts",
            Some(params),
        )
        .await
}

/// Gets usage statistics for a specific prompt.
pub async fn get_prompt_usage(
    client: &HttpClient,
    params: super::request::GetPromptRequest,
) -> Result<super::response::UsagePromptResponse, HttpError> {
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/inventions/prompts/usage",
            Some(params),
        )
        .await
}
