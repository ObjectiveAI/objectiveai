//! The `authorization_code` grant, RFC 6749 §4.1.3, with PKCE.

use axum::response::Response;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest as _, Sha256};

use super::{Grantee, Next, TokenForm, answer};
use crate::endpoints::OAuthError;
use crate::serve::Issuer;
use crate::tokens;

/// The tokens for an authorization code. The rules:
///
/// 1. **Asked for.** The form has `code`, `redirect_uri` and
///    `code_verifier`, else `invalid_request`.
/// 2. **Spent once.** The code is taken — read and removed at once — so
///    a code there is no such one of, one expired, and one spent already
///    are the one `invalid_grant`.
/// 3. **Bound.** The `redirect_uri` must be the one the code was sent
///    to, and the verifier's S256 hash the challenge it was issued for,
///    RFC 7636 §4.6, else `invalid_grant`.
/// 4. **Signed in.** The answer is an access token, a refresh token
///    beginning a new chain, and, for `openid`, an ID token carrying the
///    request's `nonce`.
pub async fn authorization_code(issuer: &Issuer, form: &TokenForm) -> Result<Response, OAuthError> {
    // Rule 1.
    let required = |name: &str| OAuthError::invalid_request(format!("`{name}` is required"));
    let code = form.code.as_deref().ok_or_else(|| required("code"))?;
    let redirect_uri = form.redirect_uri.as_deref().ok_or_else(|| required("redirect_uri"))?;
    let verifier = form.code_verifier.as_deref().ok_or_else(|| required("code_verifier"))?;
    // Rule 2.
    let refused = || OAuthError::invalid_grant("the code is not good");
    let bound = issuer.store.codes().take(&tokens::hash(code)).await?.ok_or_else(refused)?;
    // Rule 3.
    if bound.redirect_uri != redirect_uri {
        return Err(OAuthError::invalid_grant("the redirect URI is not the code's"));
    }
    if URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != bound.challenge {
        return Err(OAuthError::invalid_grant("the code verifier is not the challenge's"));
    }
    // Rule 4.
    let grantee = Grantee {
        sub: bound.subject,
        scope: bound.scope,
        audience: bound.audience,
        auth_time: Some(bound.auth_time),
        nonce: bound.nonce,
    };
    answer(issuer, grantee, Next::Begin).await
}
