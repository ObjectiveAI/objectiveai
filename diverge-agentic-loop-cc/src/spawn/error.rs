//! What the chunk stream can fail with.

use crate::response;

/// An error on the chunk stream — either the wire misbehaving, or an
/// error-typed record arriving BEFORE the first assistant message,
/// which makes it the request's own failure rather than news inside
/// a working run (those become notifications instead; the reader's
/// docs carry the rule).
///
/// The records ride whole: the root handler picks its HTTP status
/// and body from the record itself when it lands.
#[derive(Debug)]
pub enum Error {
    /// A stdout line failed the strict parse.
    Parse(serde_json::Error),
    /// Rate-limited before the first assistant message.
    RateLimit(response::rate_limit_event::RateLimitEvent),
    /// Authentication failed before the first assistant message.
    Auth(response::auth_status::AuthStatus),
    /// The run ended in error before the first assistant message.
    Result(response::result::ResultError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(error) => {
                write!(f, "a stdout line did not parse: {error}")
            }
            Error::RateLimit(_) => {
                write!(f, "rate limited before the run produced anything")
            }
            Error::Auth(status) => match &status.error {
                Some(error) => write!(
                    f,
                    "authentication failed before the run produced \
                     anything: {error}"
                ),
                None => write!(
                    f,
                    "authentication failed before the run produced \
                     anything"
                ),
            },
            Error::Result(error) => write!(
                f,
                "the run ended in error before producing anything: {}",
                error.errors.join("; ")
            ),
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
