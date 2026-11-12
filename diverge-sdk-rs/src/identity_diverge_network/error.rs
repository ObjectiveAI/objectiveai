//! Why an account could not be listed or signed in.

use std::fmt;
use std::io;
use std::path::PathBuf;

use super::Account;

/// What the [`Accounts`](super::Accounts) fail with.
#[derive(Debug)]
pub enum Error {
    /// A folder or a file could not be made, read or written.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// An account could not be written as JSON, or its file is not one
    /// this module wrote.
    Json(serde_json::Error),
    /// The word names no provider: the word, and the ones there are.
    Provider {
        /// The word given.
        named: String,
        /// Every provider's name.
        known: Vec<&'static str>,
    },
    /// A provider could not be reached, or its answer read.
    Http(reqwest::Error),
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
    /// The provider refused: its error code, and what it said.
    Refused {
        /// The error code.
        code: String,
        /// The provider's description, where it gave one.
        description: Option<String>,
    },
    /// There is no such account on this host.
    Missing(Account),
    /// The account's tokens are no longer good at its provider —
    /// ended there, or expired with nothing to renew them by: it signs
    /// in again.
    SignedOut(Account),
    /// The device code expired before a person decided.
    Expired,
    /// The person denied the sign-in.
    Denied,
    /// The provider's answer named no user, or one whose id cannot
    /// name a file: nothing was written.
    Id,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Json(error) => write!(f, "an account's file could not be written or read: {error}"),
            Error::Provider { named, known } => write!(f, "`{named}` is no provider: the providers are {}", known.join(", ")),
            Error::Http(error) => write!(f, "the provider could not be asked: {error}"),
            Error::Answer { url, status, body } => write!(f, "`{url}` answered {status} with what is not OAuth: {body}"),
            Error::Refused { code, description } => match description {
                Some(description) => write!(f, "the provider refused, `{code}`: {description}"),
                None => write!(f, "the provider refused, `{code}`"),
            },
            Error::Missing(account) => write!(f, "there is no account `{account}` on this host"),
            Error::SignedOut(account) => write!(f, "the account `{account}` is signed out at its provider; it logs in again"),
            Error::Expired => f.write_str("the sign-in expired before it was approved"),
            Error::Denied => f.write_str("the sign-in was denied"),
            Error::Id => f.write_str("the provider's answer named no user"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Json(error) => Some(error),
            Error::Http(error) => Some(error),
            _ => None,
        }
    }
}
