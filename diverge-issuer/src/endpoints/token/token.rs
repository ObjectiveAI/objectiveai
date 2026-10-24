//! The token endpoint's handler.

use std::sync::Arc;

use axum::Form;
use axum::extract::State;
use axum::extract::rejection::FormRejection;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

use super::{TokenForm, device_code, refresh_token};
use crate::endpoints::{OAuthError, client};
use crate::serve::Issuer;

/// The device code grant's `grant_type`, RFC 8628 §3.4.
const DEVICE_CODE: &str = "urn:ietf:params:oauth:grant-type:device_code";

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
    client(headers, form.client_id.as_deref(), form.client_secret.as_deref())?;
    let grant_type = form.grant_type.as_deref().ok_or_else(|| OAuthError::invalid_request("`grant_type` is required"))?;
    match grant_type {
        DEVICE_CODE => device_code(issuer, &form).await,
        "refresh_token" => refresh_token(issuer, &form).await,
        _ => Err(OAuthError::bad("unsupported_grant_type", format!("`{grant_type}` is not a grant this issuer has"))),
    }
}
