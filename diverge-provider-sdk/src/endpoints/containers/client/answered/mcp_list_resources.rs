//! `resources/list`, answered.

use bytes::Bytes;
use rmcp::model::{ErrorData, ListResourcesResult};

use super::Answered;
use crate::decode::Decode as _;
use crate::shared::mcp;

/// The resource list, once; or the server's MCP error.
#[derive(Debug, Clone, Copy)]
pub struct McpListResources;

impl Answered for McpListResources {
    type Item = ListResourcesResult;
    type Error = mcp::FrameError;
    type Refusal = ErrorData;

    fn decode(payload: Bytes) -> Result<Result<ListResourcesResult, ErrorData>, mcp::FrameError> {
        Ok(match mcp::list_resources::response::Frame::decode(&payload)? {
            mcp::list_resources::response::Frame::Result(result) => Ok(result),
            mcp::list_resources::response::Frame::Error(error) => Err(error),
        })
    }
}
