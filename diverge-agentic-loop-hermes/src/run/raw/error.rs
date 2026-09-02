//! A run that could not be driven.

use std::error;
use std::fmt;

/// Why a run could not be started or its stream read.
#[derive(Debug)]
pub enum Error {
    /// The HTTP exchange itself failed: the gateway is not listening,
    /// the connection dropped mid-body.
    Http(reqwest::Error),
    /// The gateway refused, with its status and its body verbatim —
    /// `401` auth, `400` a bad request, `404` the run gone, `429` the
    /// concurrency cap.
    Status {
        /// The HTTP status.
        status: u16,
        /// The body: JSON when it was, the text otherwise.
        body: serde_json::Value,
    },
    /// The `202` would not parse as `{"run_id", "status"}`.
    Started(serde_json::Error),
    /// A data frame that is not JSON. Every JSON shape parses — the
    /// vocabulary ends in an `Unknown` tail — so only malformed text
    /// lands here.
    Frame(serde_json::Error),
    /// The event stream broke: the SSE parser refused a line, or the
    /// socket failed under it.
    Stream(eventsource_stream::EventStreamError<reqwest::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http(error) => {
                write!(f, "the gateway could not be reached: {error}")
            }
            Error::Status { status, body } => {
                write!(f, "the gateway answered {status}: {body}")
            }
            Error::Started(error) => {
                write!(f, "the run's 202 did not parse: {error}")
            }
            Error::Frame(error) => {
                write!(f, "a run event frame is not JSON: {error}")
            }
            Error::Stream(error) => {
                write!(f, "the run's event stream broke: {error}")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Http(error) => Some(error),
            Error::Started(error) | Error::Frame(error) => Some(error),
            Error::Stream(error) => Some(error),
            Error::Status { .. } => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::Http(error)
    }
}
