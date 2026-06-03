//! HTTP functions for profile management.

use crate::{HttpClient, HttpError};

/// Lists all profiles accessible to the authenticated user.
pub async fn list_profiles(
    client: &HttpClient,
    params: super::request::ListProfilesRequest,
) -> Result<super::response::ListProfileResponse, HttpError> {
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/profiles/list",
            Some(params),
        )
        .await
}

/// Retrieves a profile definition.
pub async fn get_profile(
    client: &HttpClient,
    params: super::request::GetProfileRequest,
) -> Result<super::response::GetProfileResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "functions/profiles", Some(params))
        .await
}

/// Gets usage statistics for a specific profile.
pub async fn get_profile_usage(
    client: &HttpClient,
    params: super::request::GetProfileRequest,
) -> Result<super::response::UsageProfileResponse, HttpError> {
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/profiles/usage",
            Some(params),
        )
        .await
}
