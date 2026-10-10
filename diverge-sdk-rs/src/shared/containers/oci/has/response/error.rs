//! Why the answer could not be read.

use std::fmt;

/// A has answer that is not the byte `0` alone, or the byte `1` and a
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// No byte at all.
    Empty,
    /// A first byte that is neither `0` nor `1`.
    Byte(u8),
    /// The byte `0` with bytes after it.
    Trailing,
    /// The byte `1` with no name after it, or with bytes that are not
    /// UTF-8.
    Name,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("has answer is empty"),
            FrameError::Byte(byte) => write!(f, "has answer byte {byte} is neither 0 nor 1"),
            FrameError::Trailing => f.write_str("has answer 0 has bytes after it"),
            FrameError::Name => f.write_str("has answer 1 has no name after it, or one that is not UTF-8"),
        }
    }
}

impl std::error::Error for FrameError {}
