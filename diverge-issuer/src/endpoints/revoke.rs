//! `POST /revoke`: RFC 7009.

use std::sync::Arc;

use axum::Form;
use axum::extract::State;
use axum::extract::rejection::FormRejection;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use super::{OAuthError, client};
use crate::serve::Issuer;
use crate::tokens;

/// The request's form.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RevokeForm {
    /// The token.
    pub token: Option<String>,
    /// What kind of token it is, which is a hint the issuer does not
    /// need: only a refresh token can be revoked.
    pub token_type_hint: Option<String>,
    /// The client's id: the issuer's one app's.
    pub client_id: Option<String>,
    /// A client secret, which the one app never has: refused.
    pub client_secret: Option<String>,
}

/// `POST /revoke`: a refresh token revoked, and its whole chain with
/// it. The client is checked as at the token endpoint. The answer is
/// `200 OK`, empty, whether a chain was revoked or not — a token there
/// is no such one of and an access token alike — as RFC 7009 §2.2
/// requires; an access token is good until it expires.
pub async fn revoke(State(issuer): State<Arc<Issuer>>, headers: HeaderMap, form: Result<Form<RevokeForm>, FormRejection>) -> Response {
    match revoked(&issuer, &headers, form).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(error) => error.into_response(),
    }
}

/// The request, done.
async fn revoked(issuer: &Issuer, headers: &HeaderMap, form: Result<Form<RevokeForm>, FormRejection>) -> Result<(), OAuthError> {
    let Form(form) = form.map_err(|rejection| OAuthError::invalid_request(rejection.body_text()))?;
    client(headers, form.client_id.as_deref(), form.client_secret.as_deref())?;
    let token = form.token.as_deref().ok_or_else(|| OAuthError::invalid_request("`token` is required"))?;
    issuer.store.refresh().revoke(&tokens::hash(token)).await?;
    Ok(())
}
