//! The `authorization_code` grant.

use axum::response::Response;

use super::{Grantee, Next, TokenForm, answer};
use crate::endpoints::{OAuthError, pkce};
use crate::serve::Issuer;
use crate::store::clients::{Client, Grant};
use crate::tokens;

/// Spend the code the form names for tokens. The rules, OpenID Connect
/// Core §3.1.3.2 and RFC 7636 §4.6:
///
/// 1. The form has `code`, `redirect_uri` and `code_verifier`, else
///    `invalid_request`.
/// 2. The code is spent as it is read, whatever follows, so a code is
///    good at most once.
/// 3. The code is refused as `invalid_grant` when there is no such
///    code, when it was issued to another client, when it has
///    expired, when `redirect_uri` is not the one it was sent to, or
///    when the verifier is not the challenge's.
/// 4. A client with `refresh_token` is issued a refresh token, the
///    first of a new chain.
pub async fn authorization_code(issuer: &Issuer, client: &Client, form: &TokenForm) -> Result<Response, OAuthError> {
    if !client.may(Grant::AuthorizationCode) {
        return Err(OAuthError::unauthorized_client());
    }
    let (Some(code), Some(redirect_uri), Some(verifier)) = (&form.code, &form.redirect_uri, &form.code_verifier) else {
        return Err(OAuthError::invalid_request("`code`, `redirect_uri` and `code_verifier` are each required"));
    };
    let code = issuer.store.codes().take(&tokens::hash(code)).await?;
    let code = code.ok_or_else(|| OAuthError::invalid_grant("the code is not one the issuer holds"))?;
    if code.client != client.id || code.expires <= crate::now() {
        return Err(OAuthError::invalid_grant("the code is not this client's, or has expired"));
    }
    if &code.redirect_uri != redirect_uri {
        return Err(OAuthError::invalid_grant("`redirect_uri` is not the one the code was sent to"));
    }
    if !pkce::verify(verifier, &code.challenge) {
        return Err(OAuthError::invalid_grant("the code verifier is not the challenge's"));
    }
    let grantee = Grantee {
        sub: code.subject,
        scope: code.scope,
        audience: code.audience,
        auth_time: Some(code.auth_time),
        nonce: code.nonce,
    };
    let next = if client.may(Grant::RefreshToken) { Next::Begin } else { Next::None };
    answer(issuer, client, grantee, next).await
}
