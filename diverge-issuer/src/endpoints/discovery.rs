//! `GET /.well-known/openid-configuration`: the discovery document.

use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use openidconnect::core::{
    CoreAuthDisplay, CoreClaimName, CoreClaimType, CoreClientAuthMethod, CoreGrantType, CoreJsonWebKey,
    CoreJweContentEncryptionAlgorithm, CoreJweKeyManagementAlgorithm, CoreJwsSigningAlgorithm, CoreResponseMode,
    CoreResponseType, CoreSubjectIdentifierType,
};
use openidconnect::{
    AdditionalProviderMetadata, AuthUrl, IssuerUrl, JsonWebKeySetUrl, ProviderMetadata, ResponseTypes, Scope, TokenUrl,
    UserInfoUrl,
};
use serde::{Deserialize, Serialize};

use super::{AUTHORIZE, DEVICE_AUTHORIZATION, JWKS, REVOKE, TOKEN, USERINFO};
use crate::config::Config;
use crate::serve::Issuer;

/// What the document says beyond OpenID Connect Discovery's own
/// fields: the endpoints RFC 8628 and RFC 7009 add, as RFC 8414 names
/// them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extra {
    /// The device authorization endpoint.
    pub device_authorization_endpoint: String,
    /// The revocation endpoint.
    pub revocation_endpoint: String,
}

impl AdditionalProviderMetadata for Extra {}

/// The document's type: OpenID Connect's core vocabulary, with
/// [`Extra`].
pub type Metadata = ProviderMetadata<
    Extra,
    CoreAuthDisplay,
    CoreClientAuthMethod,
    CoreClaimName,
    CoreClaimType,
    CoreGrantType,
    CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm,
    CoreJsonWebKey,
    CoreResponseMode,
    CoreResponseType,
    CoreSubjectIdentifierType,
>;

/// The discovery document of the issuer `config` describes, as JSON:
/// made once, at the start, since nothing in it changes while the
/// issuer runs. An endpoint that is not a URL is the error, as the
/// URL parser stated it; the configuration's own check makes that a
/// start that never happens.
pub fn document(config: &Config) -> Result<String, String> {
    let url = |path: &str| config.endpoint(path.trim_start_matches('/'));
    let text = |error: url::ParseError| error.to_string();
    let metadata = Metadata::new(
        IssuerUrl::new(config.issuer()).map_err(text)?,
        AuthUrl::new(url(AUTHORIZE)).map_err(text)?,
        JsonWebKeySetUrl::new(url(JWKS)).map_err(text)?,
        vec![ResponseTypes::new(vec![CoreResponseType::None])],
        vec![CoreSubjectIdentifierType::Public],
        vec![CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256],
        Extra {
            device_authorization_endpoint: url(DEVICE_AUTHORIZATION),
            revocation_endpoint: url(REVOKE),
        },
    )
    .set_token_endpoint(Some(TokenUrl::new(url(TOKEN)).map_err(text)?))
    .set_userinfo_endpoint(Some(UserInfoUrl::new(url(USERINFO)).map_err(text)?))
    .set_scopes_supported(Some(vec![Scope::new("openid".to_string())]))
    .set_response_modes_supported(Some(vec![CoreResponseMode::Query]))
    .set_grant_types_supported(Some(vec![CoreGrantType::DeviceCode, CoreGrantType::RefreshToken]))
    .set_token_endpoint_auth_methods_supported(Some(vec![CoreClientAuthMethod::None]))
    .set_claims_supported(Some(
        ["sub", "iss", "aud", "exp", "iat", "auth_time", "at_hash"]
            .into_iter()
            .map(|claim| CoreClaimName::new(claim.to_string()))
            .collect(),
    ));
    serde_json::to_string(&metadata).map_err(|error| error.to_string())
}

/// `GET /.well-known/openid-configuration`: the document, as JSON.
pub async fn discovery(State(issuer): State<Arc<Issuer>>) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/json")], issuer.discovery.clone())
}
