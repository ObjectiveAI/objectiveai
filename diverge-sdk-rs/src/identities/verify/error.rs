//! Why a token is not accepted.

use std::fmt;

/// What [`verify`](super::KeySets::verify) fails with.
#[derive(Debug)]
pub enum VerifyError {
    /// The token is not a JWT with an `iss` and a header naming a `kid`.
    Malformed,
    /// The token's issuer is not one the caller trusts.
    Untrusted(String),
    /// The token's `alg` is not one a key set checks.
    Algorithm(String),
    /// The issuer's key set holds no key of the token's `kid`.
    UnknownKey {
        /// The issuer.
        issuer: String,
        /// The key id.
        kid: String,
    },
    /// The issuer's key set could not be fetched: why.
    Fetch(String),
    /// The signature or a claim did not check.
    Invalid(jsonwebtoken::errors::Error),
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::Malformed => f.write_str("the token is not a JWT with an issuer and a key id"),
            VerifyError::Untrusted(issuer) => write!(f, "the issuer `{issuer}` is not trusted"),
            VerifyError::Algorithm(alg) => write!(f, "the algorithm {alg} is not one a key set checks"),
            VerifyError::UnknownKey { issuer, kid } => write!(f, "`{issuer}` has no key `{kid}`"),
            VerifyError::Fetch(why) => write!(f, "the issuer's keys could not be fetched: {why}"),
            VerifyError::Invalid(error) => write!(f, "the token does not check: {error}"),
        }
    }
}

impl std::error::Error for VerifyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VerifyError::Invalid(error) => Some(error),
            _ => None,
        }
    }
}
