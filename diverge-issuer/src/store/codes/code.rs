//! One authorization code, and what it is bound to.

/// What a code is bound to, as it is issued and as it is taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    /// The subject that signed in.
    pub subject: String,
    /// The redirect URI the code was sent to, which its exchange must
    /// name again.
    pub redirect_uri: String,
    /// The PKCE S256 challenge, which its exchange's verifier must hash
    /// to.
    pub challenge: String,
    /// The nonce the request carried, for the ID token.
    pub nonce: Option<String>,
    /// The scope granted.
    pub scope: String,
    /// The audience the access token will be for.
    pub audience: String,
    /// When the subject signed in.
    pub auth_time: i64,
}
