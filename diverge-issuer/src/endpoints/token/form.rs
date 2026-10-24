//! The token request's form.

use serde::Deserialize;

/// Every parameter a token request may carry, each absent unless its
/// grant takes it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TokenForm {
    /// The grant.
    pub grant_type: Option<String>,
    /// The client's id, where it does not authenticate with `Basic`.
    pub client_id: Option<String>,
    /// The client's secret, where it does not authenticate with
    /// `Basic`.
    pub client_secret: Option<String>,
    /// The authorization code.
    pub code: Option<String>,
    /// The redirect URI the code was sent to.
    pub redirect_uri: Option<String>,
    /// The PKCE verifier.
    pub code_verifier: Option<String>,
    /// The device code.
    pub device_code: Option<String>,
    /// The refresh token.
    pub refresh_token: Option<String>,
    /// The scope asked for.
    pub scope: Option<String>,
    /// The audience asked for, RFC 8707's.
    pub resource: Option<String>,
}
