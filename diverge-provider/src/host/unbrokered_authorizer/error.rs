//! The refusal.

use std::fmt;

use crate::host::hook;

/// What [`UnbrokeredAuthorizer`](super::UnbrokeredAuthorizer) fails
/// with: the credential was refused, or accepted and held already.
/// Nothing of it reaches the peer; it is the provider's, and a
/// refusal carries the hooks that did not answer at all, so a hook
/// that is missing, broken, or writing the wrong thing is seen as
/// such rather than as a peer that was turned away.
#[derive(Debug)]
pub enum Error {
    /// The credential was accepted, but a connection holds the
    /// identity it admits, or holds the credential, already.
    Held {
        /// The identity the credential admits.
        identity: String,
    },
    /// No way in the configuration accepted the credential.
    Refused {
        /// The hooks that did not answer, each by its name with why.
        /// Empty when every way answered and every answer was no.
        failed: Vec<(String, hook::Error)>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Held { identity } => write!(f, "a connection holds the identity `{identity}` or its credential already"),
            Error::Refused { failed } => {
                f.write_str("the credential was refused")?;
                for (name, error) in failed {
                    write!(f, "; the hook `{name}` did not answer: {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for Error {}
