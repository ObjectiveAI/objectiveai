//! HTTP functions for profile management.

use crate::{HttpClient, HttpError};

/// Lists all profiles accessible to the authenticated user.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
///
/// # Returns
///
/// A list of profiles with their GitHub repository information.
pub async fn list_profiles(
    client: &HttpClient,
) -> Result<Vec<super::response::ListProfile>, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "functions/profiles", None::<String>)
        .await
}

/// Gets usage statistics for a specific profile.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `powner` - GitHub repository owner
/// * `prepository` - GitHub repository name
/// * `pcommit` - Optional Git commit SHA (uses latest if not specified)
///
/// # Returns
///
/// Usage statistics including request count, token usage, and cost.
pub async fn get_profile_usage(
    client: &HttpClient,
    powner: &str,
    prepository: &str,
    pcommit: Option<&str>,
) -> Result<super::response::UsageProfile, HttpError> {
    let path = match pcommit {
        Some(pcommit) => {
            format!(
                "functions/profiles/{}/{}/{}/usage",
                powner, prepository, pcommit
            )
        }
        None => format!("functions/profiles/{}/{}/usage", powner, prepository),
    };
    client
        .send_unary(reqwest::Method::GET, &path, None::<String>)
        .await
}
