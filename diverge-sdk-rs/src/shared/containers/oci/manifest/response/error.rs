//! Why a manifest frame could not be written or read.

use std::error;
use std::fmt;

/// A manifest frame that could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncodeError {
    /// A media type of more bytes than a two-byte length can say,
    /// carrying how many there were. Carried rather than truncated,
    /// because a length that lies is a frame that reads somebody
    /// else's bytes as its own.
    MediaTypeLength(usize),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::MediaTypeLength(len) => {
                write!(f, "manifest media type is {len} bytes, more than 65535")
            }
        }
    }
}

impl error::Error for EncodeError {}

/// A manifest frame that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// Fewer bytes than the media type's length itself, carrying
    /// however many there were.
    Short(usize),
    /// A media type length pointing past the end of the frame.
    Truncated {
        /// What the length claimed.
        need: usize,
        /// What was actually there.
        have: usize,
    },
    /// A media type that is not UTF-8.
    MediaTypeUtf8,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Short(len) => {
                write!(
                    f,
                    "manifest frame is {len} bytes, not even a media type \
                     length"
                )
            }
            FrameError::Truncated { need, have } => {
                write!(
                    f,
                    "manifest media type claims {need} bytes and {have} follow"
                )
            }
            FrameError::MediaTypeUtf8 => {
                f.write_str("manifest media type is not utf-8")
            }
        }
    }
}

impl error::Error for FrameError {}
