//! HTTP client functions for Ensemble endpoints.

use crate::{HttpClient, HttpError};

/// Lists all Ensembles that have been used by the authenticated user.
pub async fn list_ensembles(
    client: &HttpClient,
) -> Result<Vec<super::response::ListEnsemble>, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "ensembles", None::<String>)
        .await
}

/// Retrieves a specific Ensemble by its content-addressed ID.
pub async fn get_ensemble(
    client: &HttpClient,
    ensemble_id: &str,
) -> Result<super::response::GetEnsemble, HttpError> {
    client
        .send_unary(
            reqwest::Method::GET,
            &format!("ensembles/{}", ensemble_id),
            None::<String>,
        )
        .await
}

/// Retrieves usage statistics for a specific Ensemble.
pub async fn get_ensemble_usage(
    client: &HttpClient,
    ensemble_id: &str,
) -> Result<super::response::UsageEnsemble, HttpError> {
    client
        .send_unary(
            reqwest::Method::GET,
            &format!("ensembles/{}/usage", ensemble_id),
            None::<String>,
        )
        .await
}
