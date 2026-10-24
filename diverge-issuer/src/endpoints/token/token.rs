//! The token endpoint's handler.

use std::sync::Arc;

use axum::Form;
use axum::extract::State;
use axum::extract::rejection::FormRejection;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

use super::{TokenForm, authorization_code, client_credentials, device_code, refresh_token};
use crate::endpoints::{OAuthError, authenticate};
use crate::serve::Issuer;
use crate::store::clients::Grant;

/// `POST /token` — see the [table](super).
pub async fn token(State(issuer): State<Arc<Issuer>>, headers: HeaderMap, form: Result<Form<TokenForm>, FormRejection>) -> Response {
    match grant(&issuer, &headers, form).await {
        Ok(response) => response,
        Err(error) => error.into_response(),
    }
}

/// The request's grant, done.
async fn grant(issuer: &Issuer, headers: &HeaderMap, form: Result<Form<TokenForm>, FormRejection>) -> Result<Response, OAuthError> {
    let Form(form) = form.map_err(|rejection| OAuthError::invalid_request(rejection.body_text()))?;
    let client = authenticate(issuer, headers, form.client_id.as_deref(), form.client_secret.as_deref()).await?;
    let grant_type = form.grant_type.as_deref().ok_or_else(|| OAuthError::invalid_request("`grant_type` is required"))?;
    match Grant::from_grant_type(grant_type) {
        Some(Grant::AuthorizationCode) => authorization_code(issuer, &client, &form).await,
        Some(Grant::DeviceCode) => device_code(issuer, &client, &form).await,
        Some(Grant::RefreshToken) => refresh_token(issuer, &client, &form).await,
        Some(Grant::ClientCredentials) => client_credentials(issuer, &client, &form).await,
        None => Err(OAuthError::bad("unsupported_grant_type", format!("`{grant_type}` is not a grant this issuer has"))),
    }
}
