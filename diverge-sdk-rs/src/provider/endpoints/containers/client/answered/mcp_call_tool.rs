//! `tools/call`, answered.

use bytes::Bytes;
use rmcp::model::{CallToolResult, ErrorData};

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::mcp;

/// The tool's result, once; or the server's MCP error.
#[derive(Debug, Clone, Copy)]
pub struct McpCallTool;

impl Answered for McpCallTool {
    type Item = CallToolResult;
    type Error = mcp::FrameError;
    type Refusal = ErrorData;

    fn decode(payload: Bytes) -> Result<Result<CallToolResult, ErrorData>, mcp::FrameError> {
        Ok(match mcp::call_tool::response::Frame::decode(&payload)? {
            mcp::call_tool::response::Frame::Result(result) => Ok(result),
            mcp::call_tool::response::Frame::Error(error) => Err(error),
        })
    }
}
