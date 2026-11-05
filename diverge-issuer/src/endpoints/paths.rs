//! Where each endpoint is.

/// The discovery document, where OpenID Connect Discovery §4 puts it.
pub const DISCOVERY: &str = "/.well-known/openid-configuration";
/// The key set.
pub const JWKS: &str = "/jwks";
/// The authorization endpoint, which refuses every request.
pub const AUTHORIZE: &str = "/authorize";
/// The token endpoint.
pub const TOKEN: &str = "/token";
/// The revocation endpoint.
pub const REVOKE: &str = "/revoke";
/// The user info endpoint.
pub const USERINFO: &str = "/userinfo";
