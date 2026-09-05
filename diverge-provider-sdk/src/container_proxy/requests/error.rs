//! Why a frame on `/requests` could not be decoded.

use std::error;
use std::fmt;

use crate::container_proxy::vault;

/// A frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// Fewer bytes than a channel and a kind.
    Truncated,
    /// A kind this wire does not define.
    ///
    /// Every kind is fixed and enumerated, so an unfamiliar value is
    /// a malformed frame rather than a peer with more protocol than
    /// this one.
    UnknownKind(u8),
    /// MCP params that would not parse.
    Mcp(serde_json::Error),
    /// A vault operation that would not decode.
    Vault(vault::RequestError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("request frame is shorter than its header")
            }
            FrameError::UnknownKind(kind) => {
                write!(f, "unknown request kind {kind}")
            }
            FrameError::Mcp(error) => {
                write!(f, "mcp params did not parse: {error}")
            }
            FrameError::Vault(error) => {
                write!(f, "vault request did not decode: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Mcp(error) => Some(error),
            FrameError::Vault(error) => Some(error),
            FrameError::Truncated | FrameError::UnknownKind(_) => None,
        }
    }
}
