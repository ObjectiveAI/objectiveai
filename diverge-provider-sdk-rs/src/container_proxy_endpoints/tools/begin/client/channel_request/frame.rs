//! What a client's channel request frame carries for a tool container
//! begin.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::postgres;
use crate::shared::mcp;

/// What the server asks the proxy for once a tool container has
/// begun.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Postgres`](Self::Postgres) |
/// | `1` | [`Schema`](Self::Schema) |
/// | `2` | [`McpListTools`](Self::McpListTools) |
/// | `3` | [`McpListResources`](Self::McpListResources) |
/// | `4` | [`McpCallTool`](Self::McpCallTool) |
/// | `5` | [`McpReadResource`](Self::McpReadResource) |
/// | `6` | [`McpNotifications`](Self::McpNotifications) |
///
/// The first two are the same in both begin scopes, so a reader of
/// one is a reader of both; what follows is this family's own
/// exchange. All
/// of them reach INTO the container: the last hop of what a caller
/// opened on the provider.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The server's half of a database connection. Tag `0`.
    ///
    /// Opened once the server has taken the proxy's half, quoting the
    /// same connection; what comes back is everything the container's
    /// driver wrote. See
    /// [`postgres`](crate::shared::containers::postgres) for the pair.
    Postgres(postgres::request::Postgres),
    /// What the arguments may be. Tag `1`.
    ///
    /// Carries nothing — the variant is bare — and the proxy answers
    /// with the JSON Schema of the container's arguments. See
    /// [`schema`](crate::shared::containers::schema).
    Schema,
    /// What tools the container's server has. Tag `2`.
    ///
    /// The family's own exchange, and the first of five: the caller
    /// speaking to the MCP server inside, each of the exchanges in
    /// [`shared::mcp`](crate::shared::mcp) on a channel of its own,
    /// answered by the container's own server.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources it has. Tag `3`.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one of its tools. Tag `4`.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one of its resources. Tag `5`.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything it says on its own account. Tag `6`.
    ///
    /// Carries nothing, and answers for as long as the channel lives.
    McpNotifications(mcp::notifications::request::Request),
}

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 0;

/// Tag for [`Frame::Schema`].
const SCHEMA: u8 = 1;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 2;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 3;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 4;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 5;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 6;

impl Encode for Frame {
    /// The ordinary JSON failure, from whichever ask has one.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Schema => {
                out.extend_from_slice(&[SCHEMA]);
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
                request.encode(out).map_err(|error| match error {})
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Several ways to fail, and the parses among them name which.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            SCHEMA => Ok(Frame::Schema),
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

/// A tools begin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's.
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
                f.write_str("tools begin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools begin channel request tag {tag}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::McpParams(error) => {
                write!(f, "mcp request params did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::McpParams(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
