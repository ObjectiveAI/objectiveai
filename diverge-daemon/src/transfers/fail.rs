//! Why a transfer did not land.

use std::fmt;

use crate::volumes;

/// The transfer's answers that are not `Transferred`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fail {
    /// No source is the one named, or nothing is at the path.
    NotFound,
    /// The destination names an agent, a tool or a volume that is
    /// none.
    NoDestination,
    /// A volume at either end is held.
    Held,
    /// It failed, in a sentence; a file is at the destination whole
    /// or as it was, and files that landed stay.
    Error(String),
}

impl fmt::Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fail::NotFound => f.write_str("nothing is at the source"),
            Fail::NoDestination => f.write_str("the destination is none"),
            Fail::Held => f.write_str("a volume at one end is held"),
            Fail::Error(error) => f.write_str(error),
        }
    }
}

impl From<volumes::Fail> for Fail {
    fn from(fail: volumes::Fail) -> Self {
        match fail {
            volumes::Fail::NotFound => Fail::NotFound,
            volumes::Fail::Held => Fail::Held,
            volumes::Fail::Error(error) => Fail::Error(error),
        }
    }
}
