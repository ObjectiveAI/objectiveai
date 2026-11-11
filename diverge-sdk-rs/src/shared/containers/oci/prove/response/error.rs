//! Why the answer could not be read.

use std::fmt;

/// A prove answer that is not exactly 32 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The answer's length, which is not 32.
    Length(usize),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Length(length) => write!(f, "prove answer is {length} bytes, not 32"),
        }
    }
}

impl std::error::Error for FrameError {}
