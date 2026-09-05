//! Why a frame on the `/mcp/notifications` path could not be decoded.

use crate::shared::mcp;

/// A frame that could not be read: the payload would not decode as
/// a notification.
#[derive(Debug)]
pub struct FrameError(pub mcp::FrameError);

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "notification frame did not decode: {}", self.0)
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}
