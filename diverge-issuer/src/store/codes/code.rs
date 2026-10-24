//! One authorization code.

/// An authorization code, as it was issued: everything its token
/// request is checked against, and everything the tokens will say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Code {
    /// The SHA-256 of the code, as lowercase hex.
    pub hash: String,
    /// The client it was issued to.
    pub client: String,
    /// The identity that signed in.
    pub subject: String,
    /// The redirect URI it was sent to, which the token request names
    /// again.
    pub redirect_uri: String,
    /// The PKCE challenge: the unpadded base64url SHA-256 of the
    /// verifier the token request presents.
    pub challenge: String,
    /// The scope granted.
    pub scope: String,
    /// The audience the access token is for.
    pub audience: String,
    /// The client's nonce, for the ID token.
    pub nonce: Option<String>,
    /// When the identity signed in.
    pub auth_time: i64,
    /// When the code stops being good.
    pub expires: i64,
}
