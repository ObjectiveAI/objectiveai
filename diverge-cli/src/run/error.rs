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
    /// `--arguments` is not a JSON object: where in it, and what was
    /// wrong.
    Arguments(serde_path_to_error::Error<serde_json::Error>),
    /// The root's identities could not be opened: what they said.
    Identities(String),
    /// The daemon could not be dialled.
    Dial(String),
    /// The handshake did not complete: the account's token could not
    /// be had, or the daemon closed on it.
    Handshake(String),
    /// The daemon closed the connection with no answer, which is what
    /// it does to an identity it does not admit — and to nothing else
    /// it says: the account may be none of the daemon's.
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
    /// Neither a request nor a schema beneath it was asked for. Not
    /// reached by parsing — clap requires a request's `--path` unless
    /// a schema is asked for — and stated so that the absence is an
    /// error and never a panic.
    NoRequest,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Body(error) => write!(f, "the body is not what the request takes: {error}"),
            Error::Arguments(error) => write!(f, "`--arguments` is not a JSON object: {error}"),
            Error::Identities(error) => write!(f, "the identities could not be opened: {error}"),
            Error::Dial(error) => write!(f, "the daemon could not be dialled: {error}"),
            Error::Handshake(error) => write!(f, "the handshake did not complete: {error}"),
            Error::Rejected => write!(
                f,
                "the daemon closed the connection without answering: the account may be none of the daemon's"
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
            Error::NoRequest => f.write_str("no request was put, and no schema asked for"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Body(error) | Error::Arguments(error) => Some(error),
            Error::Json(error) => Some(error),
            Error::Io { source, .. } => Some(source),
            Error::Jq(error) => Some(error),
            Error::Stdout(error) => Some(error),
            Error::Rejected
            | Error::Identities(_)
            | Error::Dial(_)
            | Error::Handshake(_)
            | Error::Exchange(_)
            | Error::Timeout(_)
            | Error::NoRequest => None,
        }
    }
}

impl From<jq::Error> for Error {
    fn from(error: jq::Error) -> Self {
        Error::Jq(error)
    }
}
