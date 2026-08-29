//! What the chunk stream can fail with.

use crate::response;

/// An error on the chunk stream: the wire misbehaving, or an
/// error-typed record — yielded wherever it falls, with NO verdict
/// attached. Fatality is finality, and it is the CONSUMER's call:
/// an error before the run's first chunk is the request's own
/// failure (HTTP, by [`status`](Self::status) and
/// [`message`](Self::message)); an error the run outlives was
/// survivable news (a non-fatal notification); an error the stream
/// ends behind was the run's death (a fatal one, the last words).
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
    /// The HTTP status this failure deserves, when it is the
    /// request's own (the first-item contract): the rate limit is
    /// the one caller-visible upstream verdict; everything else —
    /// wire drift, the container's own credential dying, the run
    /// failing before it spoke — is the server's `500`.
    pub fn status(&self) -> u16 {
        match self {
            Error::RateLimit(_) => 429,
            Error::Parse(_) | Error::Auth(_) | Error::Result(_) => 500,
        }
    }

    /// The failure as an HTTP error body (or a fatal notification's
    /// message, when it arrives mid-stream) — each record's own
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
