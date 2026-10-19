//! Why a document could not be read, rewritten or patched.

use std::fmt;
use std::io;
use std::path::PathBuf;

use diverge_sdk::daemon::patch;

use crate::jq;

/// What the document operations fail with: the filesystem, the file's
/// own content, the type's refusal, jq, a patch, or stdout.
#[derive(Debug)]
pub enum Error {
    /// A file could not be read, written, renamed or removed.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// A file is not YAML.
    Parse {
        /// The file.
        path: PathBuf,
        /// Where in it, and what was wrong.
        source: serde_path_to_error::Error<serde_yaml_ng::Error>,
    },
    /// The value is not one the document's type takes: a key it does
    /// not have, a value of the wrong kind. Nothing was written.
    Invalid(serde_path_to_error::Error<serde_json::Error>),
    /// The type's own default could not be written as JSON.
    Json(serde_json::Error),
    /// The typed document could not be written as YAML.
    Serialize(serde_yaml_ng::Error),
    /// The jq program did not parse, did not compile, or failed as it
    /// ran.
    Jq(jq::Error),
    /// A `set` program yielded this many values, and one is what a
    /// document is.
    NotOne(usize),
    /// A patch's text is not a patch.
    Patch(serde_path_to_error::Error<serde_yaml_ng::Error>),
    /// A patch could not be applied: the operation, and why.
    Apply(patch::Error),
    /// Stdout could not be written.
    Stdout(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => {
                write!(f, "`{}` could not be used: {source}", path.display())
            }
            Error::Parse { path, source } => {
                write!(f, "`{}` could not be read: {source}", path.display())
            }
            Error::Invalid(error) => {
                write!(f, "the document is not one its program takes: {error}")
            }
            Error::Json(error) => write!(
                f,
                "the default document could not be written as JSON: {error}"
            ),
            Error::Serialize(error) => {
                write!(f, "the document could not be written as YAML: {error}")
            }
            Error::Jq(error) => write!(f, "{error}"),
            Error::NotOne(count) => write!(
                f,
                "the jq program yielded {count} values, and a document is one"
            ),
            Error::Patch(error) => write!(f, "the patch is not one: {error}"),
            Error::Apply(error) => write!(f, "the patch could not be applied: {error}"),
            Error::Stdout(error) => write!(f, "stdout could not be written: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Parse { source, .. } => Some(source),
            Error::Invalid(error) => Some(error),
            Error::Json(error) => Some(error),
            Error::Serialize(error) => Some(error),
            Error::Patch(error) => Some(error),
            Error::Apply(error) => Some(error),
            Error::Stdout(error) => Some(error),
            Error::Jq(error) => Some(error),
            Error::NotOne(_) => None,
        }
    }
}

impl From<jq::Error> for Error {
    fn from(error: jq::Error) -> Self {
        Error::Jq(error)
    }
}
