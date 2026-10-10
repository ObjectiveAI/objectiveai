//! What a client's channel request frame carries on a connect scope.

use std::fmt;

use crate::shared::mcp;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a client asks the daemon for while a connect is open.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes: nothing for the disconnect, the exchange's
/// request as JSON for the five MCP exchanges.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Disconnect`](Self::Disconnect) |
/// | `1` | [`McpListTools`](Self::McpListTools) |
/// | `2` | [`McpListResources`](Self::McpListResources) |
/// | `3` | [`McpCallTool`](Self::McpCallTool) |
/// | `4` | [`McpReadResource`](Self::McpReadResource) |
/// | `5` | [`McpNotifications`](Self::McpNotifications) |
///
/// All but the first reach INTO the tool's container, which is the
/// thing the client cannot reach: it runs on a provider of the serving
/// daemon's, and the serving daemon holds its run.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Leave the tool. Tag `0`.
    ///
    /// Carries nothing — the variant is bare — and nothing comes back
    /// on this channel: the daemon sends no channel response and no
    /// channel response finish on it. What comes back is the scope's
    /// finish: the scope holds the tool no more, and a tool held by
    /// nothing else stops. A disconnect of a scope that has finished
    /// already, or is finishing, changes nothing; a second disconnect
    /// changes nothing either.
    Disconnect,
    /// What tools are there. Tag `1`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See [`mcp::list_tools`](crate::shared::mcp::list_tools)
    /// for what it asks and what answers it.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources are there. Tag `2`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::list_resources`](crate::shared::mcp::list_resources) for
    /// what it asks and what answers it.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one tool. Tag `3`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See [`mcp::call_tool`](crate::shared::mcp::call_tool)
    /// for what it asks and what answers it.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one resource. Tag `4`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::read_resource`](crate::shared::mcp::read_resource) for
    /// what it asks and what answers it.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything the server says on its own account. Tag `5`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::notifications`](crate::shared::mcp::notifications) for
    /// what it asks and what answers it.
    McpNotifications(mcp::notifications::request::Request),
}

/// Tag for [`Frame::Disconnect`].
const DISCONNECT: u8 = 0;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 1;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 2;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 3;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 4;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 5;

/// A tag, then the exchange's request as JSON, if it has one.
impl Encode for Frame {
    /// The ordinary JSON failure. The disconnect cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Disconnect => {
                out.extend_from_slice(&[DISCONNECT]);
                Ok(())
            }
            Frame::McpListTools(request) => {
                out.extend_from_slice(&[MCP_LIST_TOOLS]);
                request.encode(out)
            }
            Frame::McpListResources(request) => {
                out.extend_from_slice(&[MCP_LIST_RESOURCES]);
                request.encode(out)
            }
            Frame::McpCallTool(request) => {
                out.extend_from_slice(&[MCP_CALL_TOOL]);
                request.encode(out)
            }
            Frame::McpReadResource(request) => {
                out.extend_from_slice(&[MCP_READ_RESOURCE]);
                request.encode(out)
            }
            Frame::McpNotifications(request) => {
                out.extend_from_slice(&[MCP_NOTIFICATIONS]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DISCONNECT => Ok(Frame::Disconnect),
            MCP_LIST_TOOLS => mcp::list_tools::request::Request::decode(rest)
                .map(Frame::McpListTools)
                .map_err(FrameError::McpParams),
            MCP_LIST_RESOURCES => mcp::list_resources::request::Request::decode(rest)
                .map(Frame::McpListResources)
                .map_err(FrameError::McpParams),
            MCP_CALL_TOOL => mcp::call_tool::request::Request::decode(rest)
                .map(Frame::McpCallTool)
                .map_err(FrameError::McpParams),
            MCP_READ_RESOURCE => mcp::read_resource::request::Request::decode(rest)
                .map(Frame::McpReadResource)
                .map_err(FrameError::McpParams),
            MCP_NOTIFICATIONS => Ok(Frame::McpNotifications(
                mcp::notifications::request::Request::decode(rest).unwrap_or_else(|error| match error {}),
            )),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A connect channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// One of the four MCP exchanges' params did not parse.
    ///
    /// One variant for four tags, because they fail the same way and
    /// the tag already said which was meant.
    McpParams(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools connect channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools connect channel request tag {tag}"),
            FrameError::McpParams(error) => write!(f, "mcp request params did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::McpParams(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
