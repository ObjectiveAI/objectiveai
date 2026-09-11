//! An MCP answer that could not be read.

use std::error;
use std::fmt;

/// What went wrong reading one of the five answers.
///
/// # One type for five frames
///
/// Which is unusual here: this crate writes an error per frame, so that
/// what can go wrong reading one is spelled out beside it.
///
/// These five have nothing to spell out separately. Every one is a tag
/// and then JSON, so every one fails in the same three ways, and five
/// copies would differ only in the module they sat in — while every
/// consumer that handles more than one of them would need a variant per
/// copy to say the same thing.
///
/// A caller reading a tool listing already knows it was reading a tool
/// listing. The error does not have to tell it.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of the frame's two.
    ///
    /// What a caller newer than this one produces, which is the case a
    /// tag exists to make survivable: a reader that does not know a
    /// variant says so, rather than reading somebody else's bytes as
    /// its own.
    UnknownTag(u8),
    /// The payload after the tag did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("mcp answer frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp answer frame tag {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "mcp answer did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
