//! The `refresh_token` grant, with rotation.

use axum::response::Response;

use super::{Grantee, Next, TokenForm, answer};
use crate::endpoints::{OAuthError, holds, scope};
use crate::serve::Issuer;
use crate::store::refresh::Rotation;
use crate::tokens;

/// Spend the refresh token the form names for new tokens and the next
/// refresh token. The rules — the rotation's own are
/// [`refresh`](crate::store::refresh)'s:
///
/// 1. The form has `refresh_token`, else `invalid_request`.
/// 2. A `scope` asked for holds only values the chain was granted,
///    else `invalid_scope`; absent, it is the chain's. A `resource`
///    asked for is the chain's audience, else `invalid_target`. Both
///    are checked before the token is spent.
/// 3. The token is spent and the next issued; a token refused, or
///    spent already — whose chain is revoked now — is
///    `invalid_grant`.
/// 4. The answer carries the next refresh token, and an ID token when
///    the scope holds `openid`.
pub async fn refresh_token(issuer: &Issuer, form: &TokenForm) -> Result<Response, OAuthError> {
    let presented = form.refresh_token.as_deref().ok_or_else(|| OAuthError::invalid_request("`refresh_token` is required"))?;
    let hash = tokens::hash(presented);
    let refused = || OAuthError::invalid_grant("the refresh token is not good");
    let chain = issuer.store.refresh().peek(&hash).await?.ok_or_else(refused)?;
    let asked = scope(form.scope.as_deref());
    if !asked.split(' ').filter(|value| !value.is_empty()).all(|value| holds(&chain.scope, value)) {
        return Err(OAuthError::bad("invalid_scope", "the scope asked for is more than the refresh token grants"));
    }
    if form.resource.as_deref().is_some_and(|resource| resource != chain.audience) {
        return Err(OAuthError::bad("invalid_target", "the resource is not the refresh token's audience"));
    }
    let next = tokens::mint();
    let rotation =
        issuer.store.refresh().rotate(&hash, &tokens::hash(&next), issuer.config.refresh_token_days).await?;
    let chain = match rotation {
        Rotation::Rotated(chain) => chain,
        Rotation::Reused => return Err(OAuthError::invalid_grant("the refresh token was spent already; its sign-in is revoked")),
        Rotation::Refused => return Err(refused()),
    };
    let grantee = Grantee {
        sub: chain.subject,
        scope: if asked.is_empty() { chain.scope } else { asked },
        audience: chain.audience,
        auth_time: Some(chain.auth_time),
    };
    answer(issuer, grantee, Next::Rotated(next)).await
}
