//! Why a request could not be put, or its answers printed.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::jq;

/// What [`run`](super::run) fails with.
#[derive(Debug)]
pub enum Error {
    /// No request has the name.
    NoSuchRequest(String),
    /// The body is not JSON.
    Body(serde_json::Error),
    /// The body is JSON, and not an object.
    NotAnObject,
    /// The request is one this command does not put: the MCP connect
    /// scope, held by a daemon and not a command line.
    Unsupported(String),
    /// The body is not what the request takes, in the request's own
    /// words.
    Request(Box<dyn std::error::Error + Send + Sync>),
    /// The daemon could not be dialled.
    Dial(String),
    /// The credential could not be presented.
    Authorize(String),
    /// The daemon closed the connection with no answer, which is what
    /// it does to a credential it refuses — and to nothing else it
    /// says: the account's key may have been rejected.
    Rejected,
    /// The exchange failed: the scope could not be opened, a frame
    /// could not be read.
    Exchange(String),
    /// `--path` was given to a request that takes none, or not given
    /// to one that needs it.
    Path {
        /// The request.
        path: String,
        /// Whether it was given.
        given: bool,
    },
    /// A local file could not be read, written, or made.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// A frame could not be written as JSON.
    Json(serde_json::Error),
    /// The jq filter could not be run.
    Jq(jq::Error),
    /// Stdout could not be written.
    Stdout(io::Error),
    /// The request was not complete within the seconds given, and
    /// was abandoned.
    Timeout(u64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoSuchRequest(path) => write!(f, "no request is named `{path}`"),
            Error::Body(error) => write!(f, "the body is not JSON: {error}"),
            Error::NotAnObject => write!(f, "the body is JSON, and not an object"),
            Error::Unsupported(path) => write!(f, "`{path}` is not a request this command puts"),
            Error::Request(error) => write!(f, "the body is not what the request takes: {error}"),
            Error::Dial(error) => write!(f, "the daemon could not be dialled: {error}"),
            Error::Authorize(error) => write!(f, "the credential could not be presented: {error}"),
            Error::Rejected => write!(f, "the daemon closed the connection without answering: the account's key may have been rejected"),
            Error::Exchange(error) => write!(f, "the exchange failed: {error}"),
            Error::Path { path, given: true } => write!(f, "`{path}` takes no `--path`"),
            Error::Path { path, given: false } => write!(f, "`{path}` needs `--path`, the local file or directory"),
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Json(error) => write!(f, "a frame could not be written as JSON: {error}"),
            Error::Jq(error) => write!(f, "{error}"),
            Error::Stdout(error) => write!(f, "stdout could not be written: {error}"),
            Error::Timeout(seconds) => write!(f, "the request was not complete within {seconds} seconds"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Body(error) | Error::Json(error) => Some(error),
            Error::Request(error) => Some(error.as_ref()),
            Error::Io { source, .. } => Some(source),
            Error::Jq(error) => Some(error),
            Error::Stdout(error) => Some(error),
            Error::NoSuchRequest(_)
            | Error::NotAnObject
            | Error::Rejected
            | Error::Unsupported(_)
            | Error::Dial(_)
            | Error::Authorize(_)
            | Error::Exchange(_)
            | Error::Path { .. }
            | Error::Timeout(_) => None,
        }
    }
}

impl From<jq::Error> for Error {
    fn from(error: jq::Error) -> Self {
        Error::Jq(error)
    }
}
