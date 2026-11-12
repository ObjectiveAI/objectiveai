//! Why an account could not be listed, signed in, or asked as.

use std::fmt;
use std::io;
use std::path::PathBuf;

use super::Account;
use crate::file_lock;

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
    /// An account's lock could not be taken.
    Lock(file_lock::Error),
    /// An account could not be written as JSON, or its file is not one
    /// this module wrote.
    Json(serde_json::Error),
    /// The word names no issuer: the word, and the ones there are.
    Issuer {
        /// The word given.
        named: String,
        /// Every issuer's name.
        known: Vec<&'static str>,
    },
    /// An issuer, or `identity.diverge.network`, could not be reached,
    /// or its answer read.
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
    /// The issuer refused: its error code, and what it said.
    Refused {
        /// The error code.
        code: String,
        /// The issuer's description, where it gave one.
        description: Option<String>,
    },
    /// `identity.diverge.network` refused: the status, its error code,
    /// and what it said.
    Service {
        /// The HTTP status.
        status: u16,
        /// The error code.
        code: String,
        /// The description.
        description: Option<String>,
    },
    /// Where `identity.diverge.network` is, is no URL a request can be
    /// made to: what was given.
    Address(String),
    /// The subject is none of `identity.diverge.network`'s: not
    /// `<issuer>.<id>.<name>`, or its issuer is none there is. What was
    /// given.
    Subject(String),
    /// There is no such account on this host.
    Missing(Account),
    /// The account's tokens are no longer good at its issuer — ended
    /// there, or expired with nothing to renew them by: it signs in
    /// again.
    SignedOut(Account),
    /// The device code expired before a person decided.
    Expired,
    /// The person denied the sign-in.
    Denied,
    /// The issuer's answer named no user, or one whose id cannot name
    /// a file: nothing was written.
    Id,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Lock(error) => write!(f, "an account's lock could not be taken: {error}"),
            Error::Json(error) => write!(f, "an account's file could not be written or read: {error}"),
            Error::Issuer { named, known } => write!(f, "`{named}` is no issuer: the issuers are {}", known.join(", ")),
            Error::Http(error) => write!(f, "the request could not be made: {error}"),
            Error::Answer { url, status, body } => write!(f, "`{url}` answered {status} with what could not be read: {body}"),
            Error::Refused { code, description } => match description {
                Some(description) => write!(f, "the issuer refused, `{code}`: {description}"),
                None => write!(f, "the issuer refused, `{code}`"),
            },
            Error::Service { status, code, description } => match description {
                Some(description) => write!(f, "identity.diverge.network refused, {status} `{code}`: {description}"),
                None => write!(f, "identity.diverge.network refused, {status} `{code}`"),
            },
            Error::Address(address) => write!(f, "`{address}` is no address identity.diverge.network can be asked at"),
            Error::Subject(subject) => write!(
                f,
                "`{subject}` is no subject of identity.diverge.network: one is `<issuer>.<id>.<name>`, split at its first two periods, the issuer one of {}",
                super::ISSUERS.map(|issuer| issuer.name).join(", ")
            ),
            Error::Missing(account) => write!(f, "there is no account `{account}` on this host"),
            Error::SignedOut(account) => write!(f, "the account `{account}` is signed out at its issuer; it logs in again"),
            Error::Expired => f.write_str("the sign-in expired before it was approved"),
            Error::Denied => f.write_str("the sign-in was denied"),
            Error::Id => f.write_str("the issuer's answer named no user"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Lock(error) => Some(error),
            Error::Json(error) => Some(error),
            Error::Http(error) => Some(error),
            _ => None,
        }
    }
}
