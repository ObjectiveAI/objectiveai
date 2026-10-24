//! The ways a client may get a token.

/// A grant a client may use at the token endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Grant {
    /// A person signs in on the issuer's page and the client is sent
    /// back a code: OpenID Connect Core, with PKCE.
    AuthorizationCode,
    /// A person signs in on the issuer's page with a code the client
    /// shows them: RFC 8628.
    DeviceCode,
    /// A refresh token, spent for new tokens: OAuth 2.0 §6.
    RefreshToken,
    /// The client's own secret, for a token naming the client itself:
    /// OAuth 2.0 §4.4.
    ClientCredentials,
}

impl Grant {
    /// Every grant.
    pub const ALL: [Grant; 4] = [Grant::AuthorizationCode, Grant::DeviceCode, Grant::RefreshToken, Grant::ClientCredentials];

    /// The grant as the database and the CLI name it.
    pub fn name(self) -> &'static str {
        match self {
            Grant::AuthorizationCode => "authorization_code",
            Grant::DeviceCode => "device_code",
            Grant::RefreshToken => "refresh_token",
            Grant::ClientCredentials => "client_credentials",
        }
    }

    /// The grant the database or the CLI named `name`.
    pub fn from_name(name: &str) -> Option<Grant> {
        Grant::ALL.into_iter().find(|grant| grant.name() == name)
    }

    /// The grant's `grant_type`, as a token request names it and the
    /// discovery document lists it.
    pub fn grant_type(self) -> &'static str {
        match self {
            Grant::DeviceCode => "urn:ietf:params:oauth:grant-type:device_code",
            grant => grant.name(),
        }
    }

    /// The grant a token request's `grant_type` names.
    pub fn from_grant_type(grant_type: &str) -> Option<Grant> {
        Grant::ALL.into_iter().find(|grant| grant.grant_type() == grant_type)
    }
}
