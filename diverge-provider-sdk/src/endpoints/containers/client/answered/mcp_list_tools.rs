//! `tools/list`, answered.

use bytes::Bytes;
use rmcp::model::{ErrorData, ListToolsResult};

use super::Answered;
use crate::decode::Decode as _;
use crate::shared::mcp;

/// The tool list, once; or the server's MCP error.
#[derive(Debug, Clone, Copy)]
pub struct McpListTools;

impl Answered for McpListTools {
    type Item = ListToolsResult;
    type Error = mcp::FrameError;
    type Refusal = ErrorData;

    fn decode(payload: Bytes) -> Result<Result<ListToolsResult, ErrorData>, mcp::FrameError> {
        Ok(match mcp::list_tools::response::Frame::decode(&payload)? {
            mcp::list_tools::response::Frame::Result(result) => Ok(result),
            mcp::list_tools::response::Frame::Error(error) => Err(error),
        })
    }
}
