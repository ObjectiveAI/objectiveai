//! What the chunk stream can fail with.

use crate::response;

/// An error on the chunk stream: the wire misbehaving, or an
/// error-typed record — yielded wherever it falls, with NO verdict
/// attached. Fatality is finality, and it is the CONSUMER's call:
/// an error before the run's first chunk is the request's own
/// failure (HTTP, by [`status`](Self::status) and
/// [`message`](Self::message)); an error that anything at all
/// follows was survivable news (a non-fatal notification); the one
/// the stream ends ON is the run's death (the fatal final chunk).
///
/// The records ride whole: the consumer picks what it needs from
/// the record itself.
#[derive(Debug)]
pub enum Error {
    /// A stdout line failed the strict parse.
    Parse(serde_json::Error),
    /// Requests are being refused by a rate limit.
    RateLimit(response::rate_limit_event::RateLimitEvent),
    /// Authentication failed.
    Auth(response::auth_status::AuthStatus),
    /// The run ended in error. Its bill follows it on the stream —
    /// the reader yields this first and the usage chunk right
    /// behind, so a billed failure is never the stream's last word.
    Result(response::result::ResultError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(error) => {
                write!(f, "a stdout line did not parse: {error}")
            }
            Error::RateLimit(_) => {
                write!(f, "requests are being refused by a rate limit")
            }
            Error::Auth(status) => match &status.error {
                Some(error) => {
                    write!(f, "authentication failed: {error}")
                }
                None => write!(f, "authentication failed"),
            },
            Error::Result(error) => write!(
                f,
                "the run ended in error: {}",
                error.errors.join("; ")
            ),
        }
    }
}

impl Error {
    /// The failure as a notification's message — each record's own
    /// rendering where one exists.
    pub fn message(&self) -> serde_json::Value {
        match self {
            Error::Parse(error) => serde_json::json!({
                "kind": "parse",
                "error": error.to_string(),
            }),
            Error::RateLimit(event) => event.rate_limit_info.message(),
            Error::Auth(status) => status.message(),
            Error::Result(error) => error.message(),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Parse(error) => Some(error),
            Error::RateLimit(_) | Error::Auth(_) | Error::Result(_) => {
                None
            }
        }
    }
}
