//! Why no connection to a daemon could be opened.

use std::fmt;

/// What [`connect`](super::connect) fails with: nothing to try, or
/// everything tried refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fail {
    /// No link's provider is connected now, so there is nothing to
    /// connect through.
    NoProvider,
    /// Every link whose provider is connected was tried, and none
    /// opened a connection: the last refusal, in the provider's or the
    /// wire's words.
    Refused(String),
}

impl fmt::Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fail::NoProvider => f.write_str("no provider the daemon is linked through is connected"),
            Fail::Refused(error) => write!(f, "no provider the daemon is linked through opened a connection to it: {error}"),
        }
    }
}

impl std::error::Error for Fail {}
