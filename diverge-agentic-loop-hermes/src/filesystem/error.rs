//! A preparation that could not be completed.

use std::error;
use std::fmt;
use std::io;

/// Why the filesystem could not be prepared. No variant carries a
/// secret: a contradiction names the variable, not its values.
#[derive(Debug)]
pub enum PrepareError {
    /// The request sets one environment variable to two different
    /// values — the same credential supplied twice, disagreeing.
    Contradiction {
        /// The variable both wanted.
        variable: &'static str,
    },
    /// A file or directory could not be written, or the skills
    /// directory could not be looked at.
    Io(io::Error),
    /// A document would not serialize — which plain data never
    /// fails to do, so this names a bug rather than a circumstance.
    Json(serde_json::Error),
}

impl fmt::Display for PrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrepareError::Contradiction { variable } => write!(
                f,
                "the request sets {variable} to two different values"
            ),
            PrepareError::Io(error) => {
                write!(f, "the filesystem could not be prepared: {error}")
            }
            PrepareError::Json(error) => {
                write!(f, "a document would not serialize: {error}")
            }
        }
    }
}

impl error::Error for PrepareError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            PrepareError::Io(error) => Some(error),
            PrepareError::Json(error) => Some(error),
            PrepareError::Contradiction { .. } => None,
        }
    }
}

impl From<io::Error> for PrepareError {
    fn from(error: io::Error) -> Self {
        PrepareError::Io(error)
    }
}

impl From<serde_json::Error> for PrepareError {
    fn from(error: serde_json::Error) -> Self {
        PrepareError::Json(error)
    }
}
