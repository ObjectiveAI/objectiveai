//! A delivery that could not be landed.

use std::error;
use std::fmt;
use std::io;

/// A delivery that could not be landed.
#[derive(Debug)]
pub enum IngestError {
    /// A chunk with no bytes at all, so not even a tag.
    Empty,
    /// A tag that names none of the three files.
    UnknownTag(u8),
    /// A tag lower than the one before it: the files come in
    /// ascending order, each contiguous, and this sequence was
    /// reordered or interleaved.
    Order {
        /// The tag that arrived.
        tag: u8,
        /// The tag it arrived after.
        after: u8,
    },
    /// The delivery finished without a `state.db` chunk.
    MissingStateDb,
    /// A file could not be prepared, appended to, or closed.
    Io(io::Error),
}

impl fmt::Display for IngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngestError::Empty => {
                f.write_str("a continuation chunk is empty")
            }
            IngestError::UnknownTag(tag) => {
                write!(f, "unknown continuation chunk tag {tag}")
            }
            IngestError::Order { tag, after } => write!(
                f,
                "continuation chunk tag {tag} arrived after tag {after}"
            ),
            IngestError::MissingStateDb => {
                f.write_str("the continuation carries no state.db")
            }
            IngestError::Io(error) => {
                write!(f, "the continuation could not be written: {error}")
            }
        }
    }
}

impl error::Error for IngestError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            IngestError::Io(error) => Some(error),
            IngestError::Empty
            | IngestError::UnknownTag(_)
            | IngestError::Order { .. }
            | IngestError::MissingStateDb => None,
        }
    }
}

impl From<io::Error> for IngestError {
    fn from(error: io::Error) -> Self {
        IngestError::Io(error)
    }
}
