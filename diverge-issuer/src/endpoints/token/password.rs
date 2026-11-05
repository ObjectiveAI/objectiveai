//! The password grant, RFC 6749 §4.3.

use axum::response::Response;

use super::{Grantee, Next, TokenForm, answer};
use crate::endpoints::{OAuthError, audience, scope};
use crate::password;
use crate::serve::Issuer;

/// The tokens for a subject and its password. The rules:
///
/// 1. **Both are asked for.** A request with no `username` or no
///    `password` is `invalid_request`.
/// 2. **The password is checked** against the identity's hash, in the
///    same time whether or not there is such an identity — see
///    [`password::verify`]. A wrong password, and a subject there is no
///    such identity of, are the one `invalid_grant`, which says nothing
///    of which.
/// 3. **Signed in now.** Right, the identity is signed in at this
///    moment: an access token for the scope and the audience asked
///    for, a refresh token beginning a new chain, and an ID token for
///    `openid`.
pub async fn password(issuer: &Issuer, form: &TokenForm) -> Result<Response, OAuthError> {
    // Rule 1.
    let subject = form.username.as_deref().ok_or_else(|| OAuthError::invalid_request("`username` is required"))?;
    let given = form.password.as_deref().ok_or_else(|| OAuthError::invalid_request("`password` is required"))?;
    // Rule 2.
    let hash = issuer.store.identities().password_hash(subject).await?;
    if !password::verify(given, hash).await {
        return Err(OAuthError::invalid_grant("the subject and the password do not match"));
    }
    // Rule 3.
    let grantee = Grantee {
        sub: subject.to_string(),
        scope: scope(form.scope.as_deref()),
        audience: audience(form.resource.as_deref()),
        auth_time: Some(crate::now()),
    };
    answer(issuer, grantee, Next::Begin).await
}
