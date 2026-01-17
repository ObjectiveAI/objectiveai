//! HTTP functions for function management.

use crate::{HttpClient, HttpError};

/// Lists all functions accessible to the authenticated user.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
///
/// # Returns
///
/// A list of functions with their GitHub repository information.
pub async fn list_functions(
    client: &HttpClient,
) -> Result<Vec<super::response::ListFunction>, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "functions", None::<String>)
        .await
}

/// Gets usage statistics for a specific function.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `fowner` - GitHub repository owner
/// * `frepository` - GitHub repository name
/// * `fcommit` - Optional Git commit SHA (uses latest if not specified)
///
/// # Returns
///
/// Usage statistics including request count, token usage, and cost.
pub async fn get_function_usage(
    client: &HttpClient,
    fowner: &str,
    frepository: &str,
    fcommit: Option<&str>,
) -> Result<super::response::UsageFunction, HttpError> {
    let path = match fcommit {
        Some(fcommit) => {
            format!("functions/{}/{}/{}/usage", fowner, frepository, fcommit)
        }
        None => format!("functions/{}/{}/usage", fowner, frepository),
    };
    client
        .send_unary(reqwest::Method::GET, &path, None::<String>)
        .await
}
