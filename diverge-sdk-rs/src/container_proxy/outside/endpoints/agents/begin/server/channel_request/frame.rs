//! What a server's channel request frame carries for an agent
//! container begin.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::postgres;
use crate::shared::mcp;

/// What the proxy asks the server for once an agent container
/// has begun.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Postgres`](Self::Postgres) |
/// | `1` | [`McpListTools`](Self::McpListTools) |
/// | `2` | [`McpListResources`](Self::McpListResources) |
/// | `3` | [`McpCallTool`](Self::McpCallTool) |
/// | `4` | [`McpReadResource`](Self::McpReadResource) |
/// | `5` | [`McpNotifications`](Self::McpNotifications) |
///
/// Six: everything an agent container asks of the world outside —
/// its database connections, its tool calls outward to the caller's
/// MCP servers — which the server relays to the caller as the
/// provider protocol's own channel requests. No daemon connection:
/// an agent container holds no account and asks the daemon nothing;
/// a tool container's begin has that ask and no tool call outward.
/// The fuse asks are not here; each rides the scope of the mount it
/// belongs to.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The proxy's half of a database connection the container's
    /// driver opened. Tag `0`.
    ///
    /// An id the proxy minted. What comes back is everything the
    /// database says; the server opens the other half, or declines by
    /// finishing this one with nothing before it. See
    /// [`postgres`](crate::shared::containers::postgres).
    Postgres(postgres::request::Postgres),
    /// What tools the caller's servers have. Tag `1`.
    ///
    /// The container asking OUTWARD: an agent's tool calls go to
    /// servers that live with the caller, so the five exchanges in
    /// [`shared::mcp`](crate::shared::mcp) travel this direction too.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources they have. Tag `2`.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one of their tools. Tag `3`.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one of their resources. Tag `4`.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything they say on their own account. Tag `5`.
    McpNotifications(mcp::notifications::request::Request),
}

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 0;

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

impl Encode for Frame {
    /// The JSON failure from the asks that are JSON; everything else
    /// is bytes copied.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::McpListTools(request) => {
                out.extend_from_slice(&[MCP_LIST_TOOLS]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::McpListResources(request) => {
                out.extend_from_slice(&[MCP_LIST_RESOURCES]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::McpCallTool(request) => {
                out.extend_from_slice(&[MCP_CALL_TOOL]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::McpReadResource(request) => {
                out.extend_from_slice(&[MCP_READ_RESOURCE]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::McpNotifications(request) => {
                out.extend_from_slice(&[MCP_NOTIFICATIONS]);
                request.encode(out).map_err(|error| match error {})
            }
        }
    }
}

/// A agents begin channel request that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// A JSON ask did not serialize.
    Json(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Json(error) => {
                write!(f, "channel request did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Json(error) => Some(error),
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which ask it was.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            MCP_LIST_TOOLS => mcp::list_tools::request::Request::decode(rest)
                .map(Frame::McpListTools)
                .map_err(FrameError::McpParams),
            MCP_LIST_RESOURCES => {
                mcp::list_resources::request::Request::decode(rest)
                    .map(Frame::McpListResources)
                    .map_err(FrameError::McpParams)
            }
            MCP_CALL_TOOL => mcp::call_tool::request::Request::decode(rest)
                .map(Frame::McpCallTool)
                .map_err(FrameError::McpParams),
            MCP_READ_RESOURCE => {
                mcp::read_resource::request::Request::decode(rest)
                    .map(Frame::McpReadResource)
                    .map_err(FrameError::McpParams)
            }
            MCP_NOTIFICATIONS => Ok(Frame::McpNotifications(
                mcp::notifications::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A agents begin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// One of the five MCP exchanges' params did not parse.
    ///
    /// One variant for five tags, because they fail the same way and
    /// the tag already said which was meant.
    McpParams(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agents begin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agents begin channel request tag {tag}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::McpParams(error) => {
                write!(f, "mcp request params did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::McpParams(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
