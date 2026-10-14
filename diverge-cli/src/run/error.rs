//! Why a request could not be put, or its answers printed.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::jq;

/// What [`run`](super::run) fails with. What a request takes and
/// which arguments it has are the command line's, refused by clap
/// before any of these.
#[derive(Debug)]
pub enum Error {
    /// The body is not JSON, or not what the request takes: where in
    /// it, and what was wrong.
    Body(serde_path_to_error::Error<serde_json::Error>),
    /// `--mcp-request` is not the method's params.
    McpRequest(serde_json::Error),
    /// `--mcp-request` was not given, and the method takes one.
    McpRequestMissing,
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
            Error::Body(error) => write!(f, "the body is not what the request takes: {error}"),
            Error::McpRequest(error) => {
                write!(f, "`--mcp-request` is not the method's params: {error}")
            }
            Error::McpRequestMissing => {
                write!(f, "`--mcp-request` was not given, and the method takes one")
            }
            Error::Dial(error) => write!(f, "the daemon could not be dialled: {error}"),
            Error::Authorize(error) => write!(f, "the credential could not be presented: {error}"),
            Error::Rejected => write!(
                f,
                "the daemon closed the connection without answering: the account's key may have been rejected"
            ),
            Error::Exchange(error) => write!(f, "the exchange failed: {error}"),
            Error::Io { path, source } => {
                write!(f, "`{}` could not be used: {source}", path.display())
            }
            Error::Json(error) => write!(f, "a frame could not be written as JSON: {error}"),
            Error::Jq(error) => write!(f, "{error}"),
            Error::Stdout(error) => write!(f, "stdout could not be written: {error}"),
            Error::Timeout(seconds) => {
                write!(f, "the request was not complete within {seconds} seconds")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Body(error) => Some(error),
            Error::Json(error) | Error::McpRequest(error) => Some(error),
            Error::Io { source, .. } => Some(source),
            Error::Jq(error) => Some(error),
            Error::Stdout(error) => Some(error),
            Error::Rejected
            | Error::McpRequestMissing
            | Error::Dial(_)
            | Error::Authorize(_)
            | Error::Exchange(_)
            | Error::Timeout(_) => None,
        }
    }
}

impl From<jq::Error> for Error {
    fn from(error: jq::Error) -> Self {
        Error::Jq(error)
    }
}
