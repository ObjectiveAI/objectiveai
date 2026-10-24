//! Why an identity could not be read, refreshed or signed in.

use std::fmt;
use std::io;
use std::path::PathBuf;

use super::Name;
use crate::file_lock;

/// What the [`Identities`](super::Identities) fail with.
#[derive(Debug)]
pub enum Error {
    /// A folder or a file could not be made, read or written.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// An identity's lock could not be taken.
    Lock(file_lock::Error),
    /// An identity's file is not one this module wrote.
    Json {
        /// The file.
        path: PathBuf,
        /// What was wrong.
        source: serde_json::Error,
    },
    /// There is no such identity on this host.
    Missing(Name),
    /// An issuer could not be reached, or its answer read.
    Http(reqwest::Error),
    /// An issuer's discovery document is not the issuer's, or lacks
    /// what a login needs: why.
    Discovery(String),
    /// The issuer is not an `https` URL, nor an `http` one on a
    /// loopback host.
    Issuer(String),
    /// An endpoint answered with a body that is neither its answer nor
    /// a refusal.
    Answer {
        /// The endpoint.
        url: String,
        /// The HTTP status.
        status: u16,
        /// The body, as text.
        body: String,
    },
    /// The issuer refused: its error code, and what it said.
    Refused {
        /// The error code.
        code: String,
        /// The issuer's description, where it gave one.
        description: Option<String>,
    },
    /// The identity's refresh token is no longer good: it signs in
    /// again.
    SignedOut(Name),
    /// The device code expired before a person decided.
    Expired,
    /// The person denied the device code.
    Denied,
    /// The login's answer carried no ID token, or one with no `sub`.
    Subject,
    /// The login's answer carried no refresh token.
    NoRefreshToken,
    /// The identity holds no ID token.
    NoIdToken,
    /// An access token came with no `expires_in` and no `exp`.
    NoExpiry,
    /// A token the issuer just issued is good for no longer than the
    /// time asked: the seconds it is good for.
    TooShort {
        /// The seconds the new token is good for.
        expires_in: i64,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Lock(error) => write!(f, "an identity's lock could not be taken: {error}"),
            Error::Json { path, source } => write!(f, "`{}` is not an identity: {source}", path.display()),
            Error::Missing(name) => write!(f, "there is no identity {name} on this host"),
            Error::Http(error) => write!(f, "the issuer could not be asked: {error}"),
            Error::Discovery(why) => write!(f, "the issuer's discovery document is not usable: {why}"),
            Error::Issuer(issuer) => write!(f, "`{issuer}` is not an https issuer, nor an http one on this host"),
            Error::Answer { url, status, body } => write!(f, "`{url}` answered {status} with what is not OAuth: {body}"),
            Error::Refused { code, description } => match description {
                Some(description) => write!(f, "the issuer refused, `{code}`: {description}"),
                None => write!(f, "the issuer refused, `{code}`"),
            },
            Error::SignedOut(name) => write!(f, "the identity {name} is signed out; it logs in again"),
            Error::Expired => f.write_str("the device code expired before it was approved"),
            Error::Denied => f.write_str("the device code was denied"),
            Error::Subject => f.write_str("the login's answer named no subject"),
            Error::NoRefreshToken => f.write_str("the login's answer carried no refresh token"),
            Error::NoIdToken => f.write_str("the identity holds no ID token"),
            Error::NoExpiry => f.write_str("the access token came with no expiry"),
            Error::TooShort { expires_in } => {
                write!(f, "the issuer's new token is good for {expires_in} seconds, no longer than was asked")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Lock(error) => Some(error),
            Error::Json { source, .. } => Some(source),
            Error::Http(error) => Some(error),
            _ => None,
        }
    }
}
