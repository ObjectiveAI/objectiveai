//! The servers' notifications, streamed.

use bytes::Bytes;
use rmcp::model::{ErrorData, ServerNotification};

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::mcp;

/// One notification per frame for as long as the channel lives; an
/// MCP error is the last thing said.
#[derive(Debug, Clone, Copy)]
pub struct McpNotifications;

impl Answered for McpNotifications {
    type Item = ServerNotification;
    type Error = mcp::FrameError;
    type Refusal = ErrorData;

    fn decode(payload: Bytes) -> Result<Result<ServerNotification, ErrorData>, mcp::FrameError> {
        Ok(match mcp::notifications::response::Frame::decode(&payload)? {
            mcp::notifications::response::Frame::Notification(notification) => Ok(notification),
            mcp::notifications::response::Frame::Error(error) => Err(error),
        })
    }
}
