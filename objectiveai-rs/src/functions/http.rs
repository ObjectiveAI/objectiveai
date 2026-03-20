//! HTTP functions for function management.

use crate::{HttpClient, HttpError};

/// Lists all functions accessible to the authenticated user.
pub async fn list_functions(
    client: &HttpClient,
    params: super::request::ListFunctionsRequest,
) -> Result<super::response::ListFunctionResponse, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "functions/list", Some(params))
        .await
}

/// Retrieves a function definition.
pub async fn get_function(
    client: &HttpClient,
    params: super::request::GetFunctionRequest,
) -> Result<super::response::GetFunctionResponse, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "functions", Some(params))
        .await
}

/// Gets usage statistics for a specific function.
pub async fn get_function_usage(
    client: &HttpClient,
    params: super::request::GetFunctionRequest,
) -> Result<super::response::UsageFunctionResponse, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "functions/usage", Some(params))
        .await
}

/// Lists all function-profile pairs accessible to the authenticated user.
pub async fn list_function_profile_pairs(
    client: &HttpClient,
    params: super::request::ListFunctionProfilePairsRequest,
) -> Result<super::response::ListFunctionProfilePairResponse, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "functions/profiles/pairs/list", Some(params))
        .await
}

/// Gets usage statistics for a specific function-profile pair.
pub async fn get_function_profile_pair_usage(
    client: &HttpClient,
    params: super::request::GetFunctionProfilePairUsageRequest,
) -> Result<super::response::UsageFunctionProfilePairResponse, HttpError> {
    client
        .send_unary(reqwest::Method::GET, "functions/profiles/pairs/usage", Some(params))
        .await
}

/// Estimates execution cost for a specific function-profile pair and input.
///
/// This uses historical usage data for the pair together with the provided
/// input to return a statistical cost range with confidence bounds.
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `fowner` - Function GitHub repository owner
/// * `frepository` - Function GitHub repository name
/// * `fcommit` - Optional function Git commit SHA (uses latest if not specified)
/// * `powner` - Profile GitHub repository owner
/// * `prepository` - Profile GitHub repository name
/// * `pcommit` - Optional profile Git commit SHA (uses latest if not specified)
/// * `input` - The input used to estimate Function execution cost
pub async fn estimate_function_profile_pair_cost(
    client: &HttpClient,
    fowner: &str,
    frepository: &str,
    fcommit: Option<&str>,
    powner: &str,
    prepository: &str,
    pcommit: Option<&str>,
    input: &super::executions::request::FunctionRemoteProfileRemoteRequestBodyInput,
) -> Result<
    objectiveai_api::functions::executions::cost_estimate::CostEstimateResponse,
    HttpError,
> {
    let path = match (fcommit, pcommit) {
        (Some(fcommit), Some(pcommit)) => format!(
            "functions/{}/{}/{}/profiles/{}/{}/{}/estimate",
            fowner, frepository, fcommit, powner, prepository, pcommit
        ),
        (Some(fcommit), None) => format!(
            "functions/{}/{}/{}/profiles/{}/{}/estimate",
            fowner, frepository, fcommit, powner, prepository
        ),
        (None, Some(pcommit)) => format!(
            "functions/{}/{}/profiles/{}/{}/{}/estimate",
            fowner, frepository, powner, prepository, pcommit
        ),
        (None, None) => format!(
            "functions/{}/{}/profiles/{}/{}/estimate",
            fowner, frepository, powner, prepository
        ),
    };

    let body = objectiveai_api::functions::executions::cost_estimate::CostEstimateRequestBody {
        input: input.input.clone(),
    };

    client
        .send_unary(reqwest::Method::POST, &path, Some(body))
        .await
}
