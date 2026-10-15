//! The frame, aliased.

/// The exchange's own frame: see
/// [`mcp::read_resource`](crate::shared::mcp::read_resource).
pub type Frame = crate::shared::mcp::read_resource::response::Frame;
