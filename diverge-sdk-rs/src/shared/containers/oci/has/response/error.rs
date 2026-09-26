//! Why the one byte could not be read.

use std::fmt;

/// A has answer that is not one byte of `0` or `1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// No byte at all.
    Empty,
    /// One byte, and neither `0` nor `1`.
    Byte(u8),
    /// More than one byte.
    Trailing,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("has answer is empty"),
            FrameError::Byte(byte) => write!(f, "has answer byte {byte} is neither 0 nor 1"),
            FrameError::Trailing => f.write_str("has answer has more than one byte"),
        }
    }
}

impl std::error::Error for FrameError {}
