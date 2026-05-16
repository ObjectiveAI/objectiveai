//! HTTP functions for function management.

use crate::{HttpClient, HttpError};

/// Lists all functions accessible to the authenticated user.
pub async fn list_functions(
    client: &HttpClient,
    params: super::request::ListFunctionsRequest,
) -> Result<super::response::ListFunctionResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "functions/list", Some(params))
        .await
}

/// Retrieves a function definition.
pub async fn get_function(
    client: &HttpClient,
    params: super::request::GetFunctionRequest,
) -> Result<super::response::GetFunctionResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "functions", Some(params))
        .await
}

/// Gets usage statistics for a specific function.
pub async fn get_function_usage(
    client: &HttpClient,
    params: super::request::GetFunctionRequest,
) -> Result<super::response::UsageFunctionResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "functions/usage", Some(params))
        .await
}

/// Lists all function-profile pairs accessible to the authenticated user.
pub async fn list_function_profile_pairs(
    client: &HttpClient,
    params: super::request::ListFunctionProfilePairsRequest,
) -> Result<super::response::ListFunctionProfilePairResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "functions/profiles/pairs/list", Some(params))
        .await
}

/// Gets usage statistics for a specific function-profile pair.
pub async fn get_function_profile_pair_usage(
    client: &HttpClient,
    params: super::request::GetFunctionProfilePairUsageRequest,
) -> Result<super::response::UsageFunctionProfilePairResponse, HttpError> {
    client
        .send_unary(reqwest::Method::POST, "functions/profiles/pairs/usage", Some(params))
        .await
}
