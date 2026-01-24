//! HTTP client functions for Ensemble LLM endpoints.

use crate::{HttpClient, HttpError};

/// Lists all Ensemble LLMs that have been used by the authenticated user.
pub async fn list_ensemble_llms(
    client: &HttpClient,
) -> Result<super::response::ListEnsembleLlm, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "ensemble_llms", None::<String>)
        .await
}

/// Retrieves a specific Ensemble LLM by its content-addressed ID.
pub async fn get_ensemble_llm(
    client: &HttpClient,
    ensemble_llm_id: &str,
) -> Result<super::response::GetEnsembleLlm, HttpError> {
    client
        .send_unary(
            reqwest::Method::GET,
            &format!("ensemble_llms/{}", ensemble_llm_id),
            None::<String>,
        )
        .await
}

/// Retrieves usage statistics for a specific Ensemble LLM.
pub async fn get_ensemble_llm_usage(
    client: &HttpClient,
    ensemble_llm_id: &str,
) -> Result<super::response::UsageEnsembleLlm, HttpError> {
    client
        .send_unary(
            reqwest::Method::GET,
            &format!("ensemble_llms/{}/usage", ensemble_llm_id),
            None::<String>,
        )
        .await
}
