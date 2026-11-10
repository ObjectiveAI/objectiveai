//! A preparation that could not be completed.

use std::error;
use std::fmt;
use std::io;

use super::SecretsError;

/// Why the filesystem could not be prepared. No variant carries a
/// secret's value: a contradiction names the variable, a secrets
/// failure names the secret.
#[derive(Debug)]
pub enum PrepareError {
    /// The request puts one name to two different uses — a plain
    /// variable set to two values, or a secret declared for two
    /// targets.
    Contradiction {
        /// The name both wanted.
        variable: &'static str,
    },
    /// A fed value is missing, or of the wrong shape for its target.
    Secrets(SecretsError),
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
                "the request puts {variable} to two different uses"
            ),
            PrepareError::Secrets(error) => write!(f, "{error}"),
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
            PrepareError::Secrets(error) => Some(error),
            PrepareError::Io(error) => Some(error),
            PrepareError::Json(error) => Some(error),
            PrepareError::Contradiction { .. } => None,
        }
    }
}

impl From<SecretsError> for PrepareError {
    fn from(error: SecretsError) -> Self {
        PrepareError::Secrets(error)
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
