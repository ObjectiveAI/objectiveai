//! The token endpoint's answer.

use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::endpoints::{OAuthError, OPENID, holds, no_store};
use crate::serve::Issuer;
use crate::store::clients::Client;
use crate::store::refresh::Chain;
use crate::tokens::{self, Signed};

/// What the tokens are issued for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grantee {
    /// The subject: an identity, or for `client_credentials` the
    /// client.
    pub sub: String,
    /// The scope granted.
    pub scope: String,
    /// The audience the access token is for.
    pub audience: String,
    /// When the identity signed in; `None` when no identity did, and
    /// then no ID token is issued.
    pub auth_time: Option<i64>,
    /// The client's nonce, for the ID token.
    pub nonce: Option<String>,
}

/// The refresh token an answer carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// None.
    None,
    /// The first of a new chain, issued with the answer.
    Begin,
    /// The one a rotation issued already.
    Rotated(String),
}

/// A successful token response, as RFC 6749 §5.1 and OpenID Connect
/// Core §3.1.3.3 state one.
#[derive(Debug, Clone, Serialize)]
struct Body {
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
}

/// The tokens for `grantee`, issued to `client`: an access token; an
/// ID token when an identity signed in and the scope holds `openid`;
/// and the refresh token `next` says. A new chain is kept before the
/// answer is sent, so a refresh token answered is one the issuer
/// knows.
pub async fn answer(issuer: &Issuer, client: &Client, grantee: Grantee, next: Next) -> Result<Response, OAuthError> {
    let access = tokens::access(&issuer.config, &issuer.signing, &grantee.sub, &grantee.audience, &client.id, &grantee.scope)
        .map_err(OAuthError::server)?;
    let id_token = match grantee.auth_time {
        Some(auth_time) if holds(&grantee.scope, OPENID) => {
            let signed = Signed {
                sub: &grantee.sub,
                client_id: &client.id,
                auth_time,
                nonce: grantee.nonce.as_deref(),
                access_token: &access.token,
            };
            Some(tokens::id(&issuer.config, &issuer.signing, signed).map_err(OAuthError::server)?)
        }
        _ => None,
    };
    let refresh_token = match next {
        Next::None => None,
        Next::Rotated(token) => Some(token),
        Next::Begin => {
            let token = tokens::mint();
            let chain = Chain {
                client: client.id.clone(),
                subject: grantee.sub.clone(),
                scope: grantee.scope.clone(),
                audience: grantee.audience.clone(),
                auth_time: grantee.auth_time.unwrap_or_else(crate::now),
            };
            let hash = tokens::hash(token.expose());
            issuer.store.refresh().issue(&chain, &hash, issuer.config.refresh_token_days).await?;
            Some(token.expose().to_string())
        }
    };
    let body = Body {
        access_token: access.token,
        token_type: "Bearer",
        expires_in: access.expires_in,
        refresh_token,
        id_token,
        scope: (!grantee.scope.is_empty()).then_some(grantee.scope),
    };
    Ok((no_store(), Json(body)).into_response())
}
