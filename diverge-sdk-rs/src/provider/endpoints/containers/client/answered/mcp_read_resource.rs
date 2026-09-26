//! `resources/read`, answered.

use bytes::Bytes;
use rmcp::model::{ErrorData, ReadResourceResult};

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::mcp;

/// The resource's contents, once; or the server's MCP error.
#[derive(Debug, Clone, Copy)]
pub struct McpReadResource;

impl Answered for McpReadResource {
    type Item = ReadResourceResult;
    type Error = mcp::FrameError;
    type Refusal = ErrorData;

    fn decode(payload: Bytes) -> Result<Result<ReadResourceResult, ErrorData>, mcp::FrameError> {
        Ok(match mcp::read_resource::response::Frame::decode(&payload)? {
            mcp::read_resource::response::Frame::Result(result) => Ok(result),
            mcp::read_resource::response::Frame::Error(error) => Err(error),
        })
    }
}
