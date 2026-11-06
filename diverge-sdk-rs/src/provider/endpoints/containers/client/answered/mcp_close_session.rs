//! A session's close, answered.

use bytes::Bytes;
use rmcp::model::ErrorData;

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::mcp;

/// The session closed, once; or the MCP error that kept it open.
#[derive(Debug, Clone, Copy)]
pub struct McpCloseSession;

impl Answered for McpCloseSession {
    type Item = ();
    type Error = mcp::FrameError;
    type Refusal = ErrorData;

    fn decode(payload: Bytes) -> Result<Result<(), ErrorData>, mcp::FrameError> {
        Ok(match mcp::close_session::response::Frame::decode(&payload)? {
            mcp::close_session::response::Frame::Closed => Ok(()),
            mcp::close_session::response::Frame::Error(error) => Err(error),
        })
    }
}
