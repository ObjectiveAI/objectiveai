//! The refusal.

use std::fmt;

use crate::hook;

/// What [`UnbrokeredAuthorizer`](super::UnbrokeredAuthorizer) fails
/// with: the credential was refused. Nothing of it reaches the peer;
/// it is for the provider's log, and it carries what the log wants —
/// the hooks that did not answer at all, so a hook that is missing,
/// broken, or writing the wrong thing is seen as such rather than as
/// a peer that was turned away.
#[derive(Debug)]
pub enum Error {
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
