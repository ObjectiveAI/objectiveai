//! What a client's channel request frame carries for a tool container run.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;
use crate::shared::containers::{filetree, postgres, read, write_path};

/// What a caller asks a provider for while a tool container runs.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Stop`](Self::Stop) |
/// | `1` | [`Filetree`](Self::Filetree) |
/// | `2` | [`Read`](Self::Read) |
/// | `3` | [`Write`](Self::Write) |
/// | `4` | [`Postgres`](Self::Postgres) |
/// | `5` | [`McpListTools`](Self::McpListTools) |
/// | `6` | [`McpListResources`](Self::McpListResources) |
/// | `7` | [`McpCallTool`](Self::McpCallTool) |
/// | `8` | [`McpReadResource`](Self::McpReadResource) |
/// | `9` | [`McpNotifications`](Self::McpNotifications) |
///
/// The first five are the same in every container scope, in the same
/// order, so a reader of one is a reader of all; what follows is this
/// family's own exchange. All of them but the first reach INTO the
/// container, which is the thing a caller cannot dial: it runs on the
/// provider. That is the whole reason these channels open outward
/// from the client rather than the other way.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Stop the container. Tag `0`.
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
    /// The scope IS the container's life, so dropping the connection
    /// stops it too. The difference is that a provider cannot tell a
    /// deliberate exit from a network that stopped answering, and has
    /// to wait to find out. This is unambiguous and immediate: a
    /// caller that says so is not gone, it is finished.
    ///
    /// # What it does to everyone else
    ///
    /// Ends them. Connectors hold scopes on a container that no longer
    /// exists, so those scopes finish too — a connection cannot
    /// outlive the thing it joined.
    Stop,
    /// The container's filesystem, watched. Tag `1`.
    ///
    /// Carries nothing; the provider answers with a snapshot and then
    /// every change, for as long as the channel lives. See
    /// [`filetree`](crate::shared::containers::filetree).
    Filetree(filetree::request::Request),
    /// One file, read out of the container. Tag `2`.
    ///
    /// See [`read`](crate::shared::containers::read) for why this is
    /// one file and never a directory.
    Read(read::request::Request),
    /// One file, written into the container. Tag `3`.
    ///
    /// Carries no content. The provider answers by opening a channel
    /// of its own asking for it — see
    /// [`write_path`](crate::shared::containers::write_path) for why
    /// it travels that direction, and
    /// [`write_bytes`](crate::shared::containers::write_bytes) for
    /// what comes back.
    Write(write_path::request::Request),
    /// The caller's half of a database connection. Tag `4`.
    ///
    /// Opened once the caller has taken the provider's half, quoting
    /// the same connection; what comes back is everything the
    /// container wrote. See
    /// [`postgres`](crate::shared::containers::postgres) for the pair.
    Postgres(postgres::request::Postgres),
    /// What tools are there. Tag `5`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See [`mcp::list_tools`](crate::shared::mcp::list_tools)
    /// for what it asks and what answers it.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources are there. Tag `6`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::list_resources`](crate::shared::mcp::list_resources) for
    /// what it asks and what answers it.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one tool. Tag `7`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See [`mcp::call_tool`](crate::shared::mcp::call_tool)
    /// for what it asks and what answers it.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one resource. Tag `8`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::read_resource`](crate::shared::mcp::read_resource) for
    /// what it asks and what answers it.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything the server says on its own account. Tag `9`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::notifications`](crate::shared::mcp::notifications) for
    /// what it asks and what answers it.
    McpNotifications(mcp::notifications::request::Request),
}

/// Tag for [`Frame::Stop`].
const STOP: u8 = 0;

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 1;

/// Tag for [`Frame::Read`].
const READ: u8 = 2;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 3;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 4;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 5;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 6;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 7;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 8;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 9;

impl Encode for Frame {
    /// The ordinary JSON failure, from whichever half has one.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Stop => {
                out.extend_from_slice(&[STOP]);
                Ok(())
            }
            Frame::Filetree(request) => {
                out.extend_from_slice(&[FILETREE]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Read(request) => {
                out.extend_from_slice(&[READ]);
                request.encode(out)
            }
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                request.encode(out)
            }
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                request.encode(out).map_err(|error| match error {})
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
    /// Several ways to fail, and the parses among them name which.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            STOP => Ok(Frame::Stop),
            FILETREE => Ok(Frame::Filetree(
                filetree::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            READ => read::request::Request::decode(rest)
                .map(Frame::Read)
                .map_err(FrameError::Read),
            WRITE => write_path::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
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

/// A tool container run channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's.
    UnknownTag(u8),
    /// The read request did not parse.
    Read(serde_json::Error),
    /// The write request did not parse.
    Write(serde_json::Error),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// One of the five MCP exchanges' params did not parse.
    ///
    /// One variant for five tags, because they fail the same way and
    /// the tag already said which was meant. Naming each would be five
    /// cases every reader matches and none of them distinguishes
    /// anything a caller could act on.
    McpParams(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("tools run channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools run channel request tag {tag}")
            }
            FrameError::Read(error) => {
                write!(f, "read request did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write request did not parse: {error}")
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
            FrameError::Read(error)
            | FrameError::Write(error)
            | FrameError::McpParams(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
