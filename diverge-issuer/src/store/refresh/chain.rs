//! One chain: a sign-in a refresh token carries forward.

/// What a chain grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// The client the chain was issued to.
    pub client: String,
    /// The identity that signed in.
    pub subject: String,
    /// The scope granted.
    pub scope: String,
    /// The audience the access tokens are for.
    pub audience: String,
    /// When the identity signed in.
    pub auth_time: i64,
}
