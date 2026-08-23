//! What a client's channel request frame carries for an MCP plugin.

use std::error::Error;
use std::fmt;

use super::Postgres;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;

/// What a caller asks a provider for while a plugin runs.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Stop`](Self::Stop) |
/// | `1` | [`Postgres`](Self::Postgres) |
/// | `2` | [`McpListTools`](Self::McpListTools) |
/// | `3` | [`McpListResources`](Self::McpListResources) |
/// | `4` | [`McpCallTool`](Self::McpCallTool) |
/// | `5` | [`McpReadResource`](Self::McpReadResource) |
/// | `6` | [`McpNotifications`](Self::McpNotifications) |
///
/// # Five of them are MCP
///
/// And they are the whole of it. There was a sixth that tunneled an
/// HTTP exchange, which was the only way to reach the plugin's MCP
/// server before these existed; they say what is being asked instead
/// of carrying a request that says it, and between them they cover
/// everything the tunnel could do — see
/// [`shared::mcp`](crate::shared::mcp).
///
/// # Two reach into the container, and one does not
///
/// The five and [`Stop`](Self::Stop) are aimed at the thing a caller
/// cannot dial: the container runs on the provider, on a port the
/// provider published to its own loopback and told nobody.
///
/// [`Postgres`](Self::Postgres) opens outward for a different reason,
/// and the difference is worth keeping. It asks for bytes the provider
/// is already holding, so topology has nothing to do with it — the
/// writes have to arrive as a RESPONSE stream, because only a
/// responder can finish a channel and the provider needs to be able to
/// say the plugin has gone.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Stop the container. Tag `1`.
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
    /// The scope IS the plugin's life, so dropping the connection
    /// stops it too. The difference is that a provider cannot tell a
    /// deliberate exit from a network that stopped answering, and has
    /// to wait to find out. This is unambiguous and immediate: a
    /// caller that says so is not gone, it is finished.
    ///
    /// # It ends nobody else
    ///
    /// Unlike a
    /// [`laboratory's`](crate::endpoints::laboratories::run::client::channel_request::Frame::Stop),
    /// which takes every connector down with it. A plugin has no
    /// connectors and no
    /// [`id`](crate::endpoints::laboratories::run::server::response::Frame::Id)
    /// by which one could have arrived, so the caller that created it
    /// is the only party to the container's existence and stopping it
    /// concerns nobody else.
    ///
    /// # What it does to exchanges in flight
    ///
    /// Ends them, unanswered. A caller with MCP channels still open
    /// when it sends this will see them finish without heads, because
    /// the container they were aimed at is gone. Waiting for them
    /// first is the caller's to do, and nothing here does it on the
    /// caller's behalf — a provider that tried would be guessing which
    /// of a caller's outstanding calls it still wanted.
    Stop,
    /// The plugin's half of a database connection. Tag `2`.
    ///
    /// Sent in answer to a
    /// [`server::channel_request::Frame::Postgres`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Postgres),
    /// quoting the connection it names. What comes back is everything
    /// the plugin writes; the finish says the plugin's socket ended.
    ///
    /// See [`Postgres`] for why a connection takes two channels and
    /// what a caller owes the provider once it has taken the first.
    Postgres(Postgres),
    /// What tools are there. Tag `3`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::list_tools`](crate::shared::mcp::list_tools) for what it asks
    /// and what answers it.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources are there. Tag `4`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::list_resources`](crate::shared::mcp::list_resources) for what it asks
    /// and what answers it.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one tool. Tag `5`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::call_tool`](crate::shared::mcp::call_tool) for what it asks
    /// and what answers it.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one resource. Tag `6`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::read_resource`](crate::shared::mcp::read_resource) for what it asks
    /// and what answers it.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything the server says on its own account. Tag `7`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::notifications`](crate::shared::mcp::notifications) for what it asks
    /// and what answers it.
    McpNotifications(mcp::notifications::request::Request),
}

/// Tag for [`Frame::Stop`].
const STOP: u8 = 0;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 1;

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
    /// The ordinary JSON failure, from the only variant that has one.
    /// A stop carries nothing and a connection id is four known bytes.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Stop => {
                out.extend_from_slice(&[STOP]);
                Ok(())
            }
            Frame::Postgres(postgres) => {
                out.extend_from_slice(&[POSTGRES]);
                postgres
                    .encode(out)
                    .unwrap_or_else(|error| match error {});
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
    /// Five ways to fail, and two of them are JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            STOP => Ok(Frame::Stop),
            POSTGRES => Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
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

/// An MCP plugin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's eight.
    UnknownTag(u8),
    /// One of the five MCP exchanges' params did not parse.
    ///
    /// One variant for five tags, because they fail the same way and
    /// the tag already said which was meant. Naming each would be five
    /// cases every reader matches and none of them distinguishes
    /// anything a caller could act on.
    McpParams(serde_json::Error),
    /// The write request was not a connection id.
    Postgres(super::postgres::PostgresError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("mcp plugin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp plugin channel request tag {tag}")
            }
            FrameError::McpParams(error) => {
                write!(f, "mcp request params did not parse: {error}")
            }
            FrameError::Postgres(error) => {
                write!(f, "postgres write request did not parse: {error}")
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
