//! Why a `_meta` is not what the daemon attests.

use std::fmt;

/// A `_meta` read as what the daemon attests, and found otherwise:
/// which key, and what was wrong with it.
#[derive(Debug)]
pub enum Error {
    /// The key is not there, and the read needs it.
    Missing(&'static str),
    /// The key is there, and its value is not the type the key
    /// carries.
    Invalid {
        /// The key.
        key: &'static str,
        /// What was wrong with the value.
        source: serde_json::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing(key) => write!(f, "`_meta` has no `{key}`"),
            Error::Invalid { key, source } => {
                write!(f, "`_meta`'s `{key}` is not what it carries: {source}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Missing(_) => None,
            Error::Invalid { source, .. } => Some(source),
        }
    }
}
