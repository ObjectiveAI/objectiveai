//! Which MCP exchange the container asks for.

use std::error;
use std::fmt;

use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;

/// One MCP exchange, as the container asks it.
///
/// A payload leads with one byte saying which, and the rest is that
/// exchange's own params as [`shared::mcp`](crate::shared::mcp)
/// defines them.
///
/// | tag | exchange |
/// |-----|----------|
/// | `0` | [`McpListTools`](Self::McpListTools) |
/// | `1` | [`McpListResources`](Self::McpListResources) |
/// | `2` | [`McpCallTool`](Self::McpCallTool) |
/// | `3` | [`McpReadResource`](Self::McpReadResource) |
/// | `4` | [`McpNotifications`](Self::McpNotifications) |
///
/// The five are what an MCP server is once the transport is taken
/// off it: a client POSTs a JSON-RPC message for the first four and
/// opens a stream with a bare `GET` for the fifth. The verb is the
/// whole of the distinction there, and the tag is the whole of it
/// here.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// What tools are there. Tag `0`. Answered once by a
    /// [`list_tools`](mcp::list_tools) response.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources are there. Tag `1`. Answered once by a
    /// [`list_resources`](mcp::list_resources) response.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one tool. Tag `2`. Answered once by a
    /// [`call_tool`](mcp::call_tool) response.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one resource. Tag `3`. Answered once by a
    /// [`read_resource`](mcp::read_resource) response.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything the server says on its own account. Tag `4`.
    /// Carries nothing — the empty payload is the request, whole —
    /// and is answered with one
    /// [`notifications`](mcp::notifications) response per
    /// notification for as long as the channel lives.
    McpNotifications(mcp::notifications::request::Request),
}

const MCP_LIST_TOOLS: u8 = 0;
const MCP_LIST_RESOURCES: u8 = 1;
const MCP_CALL_TOOL: u8 = 2;
const MCP_READ_RESOURCE: u8 = 3;
const MCP_NOTIFICATIONS: u8 = 4;

impl Encode for Request {
    /// The ordinary JSON failure. Every params variant is serialized,
    /// the tag cannot fail, and the notification stream carries
    /// nothing to fail on.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Request::McpListTools(request) => {
                out.extend_from_slice(&[MCP_LIST_TOOLS]);
                request.encode(out)
            }
            Request::McpListResources(request) => {
                out.extend_from_slice(&[MCP_LIST_RESOURCES]);
                request.encode(out)
            }
            Request::McpCallTool(request) => {
                out.extend_from_slice(&[MCP_CALL_TOOL]);
                request.encode(out)
            }
            Request::McpReadResource(request) => {
                out.extend_from_slice(&[MCP_READ_RESOURCE]);
                request.encode(out)
            }
            Request::McpNotifications(request) => {
                out.extend_from_slice(&[MCP_NOTIFICATIONS]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
        }
    }
}

impl Request {
    /// Decode one request from the bytes after the channel byte.
    pub fn decode(bytes: &[u8]) -> Result<Self, RequestError> {
        let (tag, rest) = bytes.split_first().ok_or(RequestError::Empty)?;
        match *tag {
            MCP_LIST_TOOLS => mcp::list_tools::request::Request::decode(rest)
                .map(Request::McpListTools)
                .map_err(RequestError::Body),
            MCP_LIST_RESOURCES => {
                mcp::list_resources::request::Request::decode(rest)
                    .map(Request::McpListResources)
                    .map_err(RequestError::Body)
            }
            MCP_CALL_TOOL => mcp::call_tool::request::Request::decode(rest)
                .map(Request::McpCallTool)
                .map_err(RequestError::Body),
            MCP_READ_RESOURCE => {
                mcp::read_resource::request::Request::decode(rest)
                    .map(Request::McpReadResource)
                    .map_err(RequestError::Body)
            }
            MCP_NOTIFICATIONS => Ok(Request::McpNotifications(
                mcp::notifications::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            tag => Err(RequestError::UnknownTag(tag)),
        }
    }
}

/// An MCP request that could not be read.
#[derive(Debug)]
pub enum RequestError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this request's five.
    UnknownTag(u8),
    /// The params after the tag did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Empty => f.write_str("mcp request is empty"),
            RequestError::UnknownTag(tag) => {
                write!(f, "unknown mcp request tag {tag}")
            }
            RequestError::Body(error) => {
                write!(f, "mcp request did not parse: {error}")
            }
        }
    }
}

impl error::Error for RequestError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            RequestError::Body(error) => Some(error),
            RequestError::Empty | RequestError::UnknownTag(_) => None,
        }
    }
}
