//! What a client's channel request frame carries for a connection.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;
use crate::shared::container::{read, transfer, write_path};

/// What a connector asks a provider for while it is attached.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Read`](Self::Read) |
/// | `1` | [`Write`](Self::Write) |
/// | `2` | [`Transfer`](Self::Transfer) |
/// | `3` | [`Disconnect`](Self::Disconnect) |
/// | `4` | [`McpListTools`](Self::McpListTools) |
/// | `5` | [`McpListResources`](Self::McpListResources) |
/// | `6` | [`McpCallTool`](Self::McpCallTool) |
/// | `7` | [`McpReadResource`](Self::McpReadResource) |
/// | `8` | [`McpNotifications`](Self::McpNotifications) |
///
/// All of them but the fourth reach INTO the container, which is the
/// thing a caller cannot dial: it runs on the provider. That is the
/// whole reason these channels open outward from the client rather
/// than the other way.
///
/// # Five of them are MCP
///
/// And they are the whole of it. There was a sixth that tunneled an
/// HTTP exchange, which was the only way to reach an MCP server before
/// these existed; they say what is being asked instead of carrying a
/// request that says it, and between them they cover everything the
/// tunnel could do — see [`shared::mcp`](crate::shared::mcp).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One file, read out of the container.
    ///
    /// See [`read`](crate::shared::container::read) for why this is
    /// one file and never a directory.
    Read(read::request::Request),
    /// One file, written into the container.
    ///
    /// Carries no content. The provider answers by opening a channel
    /// of its own asking for it — see
    /// [`write_path`](crate::shared::container::write_path) for why it
    /// travels that direction, and
    /// [`write_bytes`](crate::shared::container::write_bytes) for what
    /// comes back.
    Write(write_path::request::Request),
    /// One file, moved out of this container into another without
    /// ever leaving the provider.
    ///
    /// Carries no content and receives none. See
    /// [`transfer`](crate::shared::container::transfer) for when this
    /// is available and what to do when it is not.
    Transfer(transfer::request::Request),
    /// Leave the container. Tag `4`.
    ///
    /// # It has no answer, and does not need one
    ///
    /// Nothing comes back on this channel. What comes back is the end
    /// of the SCOPE — a
    /// [`ResponseFinish`](crate::frame::server::ServerFrame::ResponseFinish),
    /// which already means nothing bearing this scope follows on any
    /// channel. Finishing this one first would be a smaller way of
    /// saying the same thing, moments earlier.
    ///
    /// # What it adds over closing the connection
    ///
    /// Leaving ends this scope either way. The difference is that a
    /// provider cannot tell a deliberate exit from a network that
    /// stopped answering, and has to wait to find out — during which
    /// the runner has not been told this connector is gone, because
    /// the provider does not yet know. This is unambiguous and
    /// immediate, and the
    /// [`Disconnected`](crate::endpoints::laboratories::run::server::response::Frame::Disconnected)
    /// follows straight away.
    ///
    /// # What it does not do
    ///
    /// Stop the container. That belongs to whoever created it, and is
    /// [`Stop`](crate::endpoints::laboratories::run::client::channel_request::Frame::Stop)
    /// on their scope. A connector leaving takes nothing with it: the
    /// container runs, other connectors stay, and the runner sees one
    /// fewer connection.
    Disconnect,
    /// What tools are there. Tag `5`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::list_tools`](crate::shared::mcp::list_tools) for what it asks
    /// and what answers it.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources are there. Tag `6`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::list_resources`](crate::shared::mcp::list_resources) for what it asks
    /// and what answers it.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one tool. Tag `7`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::call_tool`](crate::shared::mcp::call_tool) for what it asks
    /// and what answers it.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one resource. Tag `8`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::read_resource`](crate::shared::mcp::read_resource) for what it asks
    /// and what answers it.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything the server says on its own account. Tag `9`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::notifications`](crate::shared::mcp::notifications) for what it asks
    /// and what answers it.
    McpNotifications(mcp::notifications::request::Request),
}

/// Tag for [`Frame::Read`].
const READ: u8 = 0;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 1;

/// Tag for [`Frame::Transfer`].
const TRANSFER: u8 = 2;

/// Tag for [`Frame::Disconnect`].
const DISCONNECT: u8 = 3;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 4;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 5;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 6;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 7;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 8;

impl Encode for Frame {
    /// The ordinary JSON failure, from whichever half is present.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Read(request) => {
                out.extend_from_slice(&[READ]);
                request.encode(out)
            }
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                request.encode(out)
            }
            Frame::Transfer(request) => {
                out.extend_from_slice(&[TRANSFER]);
                request.encode(out)
            }
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
    /// Seven ways to fail, and the parses among them name which
    /// failed.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            READ => read::request::Request::decode(rest)
                .map(Frame::Read)
                .map_err(FrameError::Read),
            WRITE => write_path::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
            TRANSFER => transfer::request::Request::decode(rest)
                .map(Frame::Transfer)
                .map_err(FrameError::Transfer),
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
                mcp::notifications::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A connection channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's ten.
    UnknownTag(u8),
    /// One of the five MCP exchanges' params did not parse.
    ///
    /// One variant for five tags, because they fail the same way and
    /// the tag already said which was meant. Naming each would be five
    /// cases every reader matches and none of them distinguishes
    /// anything a caller could act on.
    McpParams(serde_json::Error),
    /// The read request did not parse.
    Read(serde_json::Error),
    /// The write request did not parse.
    Write(serde_json::Error),
    /// The transfer request did not parse.
    Transfer(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("connection channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown connection channel request tag {tag}")
            }
            FrameError::McpParams(error) => {
                write!(f, "mcp request params did not parse: {error}")
            }
            FrameError::Read(error) => {
                write!(f, "read request did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write request did not parse: {error}")
            }
            FrameError::Transfer(error) => {
                write!(f, "transfer request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::McpParams(error)
            | FrameError::Read(error)
            | FrameError::Write(error)
            | FrameError::Transfer(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
