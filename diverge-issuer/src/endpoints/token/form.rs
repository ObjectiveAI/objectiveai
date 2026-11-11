//! The token request's form.

use serde::Deserialize;

/// Every parameter a token request may carry, each absent unless its
/// grant takes it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TokenForm {
    /// The grant.
    pub grant_type: Option<String>,
    /// The client's id: the issuer's one app's.
    pub client_id: Option<String>,
    /// A client secret, which the one app never has: refused.
    pub client_secret: Option<String>,
    /// The authorization code.
    pub code: Option<String>,
    /// The redirect URI the code was sent to.
    pub redirect_uri: Option<String>,
    /// The PKCE verifier, RFC 7636.
    pub code_verifier: Option<String>,
    /// The refresh token.
    pub refresh_token: Option<String>,
    /// The scope asked for.
    pub scope: Option<String>,
    /// The audience asked for, RFC 8707's.
    pub resource: Option<String>,
}
