//! The ID token.

use chrono::{DateTime, Utc};
use openidconnect::core::{CoreIdToken, CoreIdTokenClaims, CoreJwsSigningAlgorithm};
use openidconnect::{AccessToken, Audience, EmptyAdditionalClaims, IssuerUrl, StandardClaims, SubjectIdentifier};

use super::Error;
use crate::config::Config;
use crate::signing::Signing;

/// What an ID token says, beyond what every one does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signed<'a> {
    /// The identity signed in.
    pub sub: &'a str,
    /// The client the token is for, its one audience: always the
    /// issuer's one app.
    pub client_id: &'a str,
    /// When the identity signed in, in seconds since the Unix epoch.
    pub auth_time: i64,
    /// The access token issued beside it, whose hash is `at_hash`.
    pub access_token: &'a str,
}

/// Sign an ID token, as OpenID Connect Core §2 states one: `iss`,
/// `sub`, `aud` the client, `exp` as an access token's, `iat` now,
/// `auth_time`, and `at_hash`, the access token's. There is no
/// `nonce`: no authorization request ever sends one.
pub fn id(config: &Config, signing: &Signing, signed: Signed<'_>) -> Result<String, Error> {
    let now = Utc::now();
    let lifetime = chrono::Duration::seconds(i64::try_from(config.access_token_seconds).unwrap_or(i64::MAX));
    let issuer = IssuerUrl::new(config.issuer()).map_err(|error| Error::Issuer(error.to_string()))?;
    let auth_time = DateTime::<Utc>::from_timestamp(signed.auth_time, 0).unwrap_or(now);
    let claims = CoreIdTokenClaims::new(
        issuer,
        vec![Audience::new(signed.client_id.to_string())],
        now + lifetime,
        now,
        StandardClaims::new(SubjectIdentifier::new(signed.sub.to_string())),
        EmptyAdditionalClaims {},
    )
    .set_auth_time(Some(auth_time));
    let access_token = AccessToken::new(signed.access_token.to_string());
    let token = CoreIdToken::new(
        claims,
        &signing.current().id,
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        Some(&access_token),
        None,
    )
    .map_err(|error| Error::Id(error.to_string()))?;
    Ok(token.to_string())
}
