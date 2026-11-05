//! Why no connection to a daemon could be opened.

use std::fmt;

/// What [`connect`](super::connect) fails with: nothing to try, or the
/// one tried refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fail {
    /// No connected provider's discover listing holds the remote: it is
    /// not discoverable on any provider this daemon is connected to.
    NoProvider,
    /// The provider chosen did not open a connection: its refusal, in the
    /// provider's or the wire's words.
    Refused(String),
}

impl fmt::Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fail::NoProvider => f.write_str("the daemon is discoverable on no provider this daemon is connected to"),
            Fail::Refused(error) => write!(f, "the provider did not open a connection to the daemon: {error}"),
        }
    }
}

impl std::error::Error for Fail {}
