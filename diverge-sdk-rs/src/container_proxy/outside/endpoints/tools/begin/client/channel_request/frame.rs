//! What a client's channel request frame carries for a tool container
//! begin.

use std::error::Error;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::postgres;
use crate::shared::containers::daemon;
use crate::shared::containers::secrets::Values;
use crate::shared::mcp;
use crate::shared::mcp::session::Sessioned;

/// What the server asks the proxy for once a tool container has
/// begun.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Postgres`](Self::Postgres) |
/// | `1` | [`Info`](Self::Info) |
/// | `2` | [`Daemon`](Self::Daemon) |
/// | `3` | [`McpListTools`](Self::McpListTools) |
/// | `4` | [`McpListResources`](Self::McpListResources) |
/// | `5` | [`McpCallTool`](Self::McpCallTool) |
/// | `6` | [`McpReadResource`](Self::McpReadResource) |
/// | `7` | [`McpNotifications`](Self::McpNotifications) |
/// | `8` | [`Secrets`](Self::Secrets) |
///
/// The first two are the same in both begin scopes, so a reader of
/// one is a reader of both; what follows is this family's own
/// exchange, and then the feed, which both scopes have again under a
/// tag of their own. All of them reach INTO the container: the last
/// hop of what a caller opened on the provider, or of what the caller
/// fed.
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
    /// with the image's DIVERGE.md, parsed — read from the
    /// container's root, the program never asked — or that it has
    /// none. See [`info`](crate::shared::containers::info).
    Info,
    /// A daemon connection the program opened, by the id the proxy
    /// announced: the server's half of it. Tag `2`.
    ///
    /// What comes back on it is everything the PROGRAM sends, as
    /// [`daemon::client::Frame`](crate::shared::containers::daemon::client::Frame)s;
    /// its finish is the program's socket ended. See
    /// [`daemon`](crate::shared::containers::daemon) for the pair.
    Daemon(daemon::request::Daemon),
    /// What tools the container's server has. Tag `3`.
    ///
    /// The family's own exchange, and the first of five: the caller
    /// speaking to the MCP server inside, each of the exchanges in
    /// [`shared::mcp`](crate::shared::mcp) on a channel of its own,
    /// answered by the container's own server — each
    /// [`Sessioned`]: on the session of the caller it names, which the
    /// proxy opens on its first exchange, as
    /// [`session`](crate::shared::mcp::session) states.
    McpListTools(Sessioned<mcp::list_tools::request::Request>),
    /// What resources it has. Tag `4`.
    McpListResources(Sessioned<mcp::list_resources::request::Request>),
    /// Run one of its tools. Tag `5`.
    McpCallTool(Sessioned<mcp::call_tool::request::Request>),
    /// Read one of its resources. Tag `6`.
    McpReadResource(Sessioned<mcp::read_resource::request::Request>),
    /// Everything it says on its own account, on the session named.
    /// Tag `7`.
    ///
    /// Carries the session alone, and answers for as long as the
    /// channel lives with what the server sends on that session.
    McpNotifications(Sessioned<mcp::notifications::request::Request>),
    /// The secrets' values, fed to the program. Tag `8`.
    ///
    /// Carries the [`Values`] as JSON — every name the program
    /// declared, each to its value — which the proxy hands the
    /// program on `POST /secrets` and holds until that call answers.
    /// Opened by the server once before it sends the run's id and
    /// again at every activation, relaying the caller's own feed.
    /// Answered once — by a
    /// [`feed::response::Frame`](crate::shared::containers::secrets::feed::response::Frame),
    /// `Fed` or the program's refusal — and then the finish; a `Fed`
    /// opens the gate the proxy holds every exchange behind. See
    /// [`secrets`](crate::shared::containers::secrets).
    Secrets(Values),
}

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 0;

/// Tag for [`Frame::Info`].
const INFO: u8 = 1;

/// Tag for [`Frame::Daemon`].
const DAEMON: u8 = 2;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 3;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 4;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 5;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 6;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 7;

/// Tag for [`Frame::Secrets`].
const SECRETS: u8 = 8;

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
            Frame::Info => {
                out.extend_from_slice(&[INFO]);
                Ok(())
            }
            Frame::Daemon(request) => {
                out.extend_from_slice(&[DAEMON]);
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
                request.encode(out)
            }
            Frame::Secrets(values) => {
                out.extend_from_slice(&[SECRETS]);
                serde_json::to_writer(out, values)
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
            INFO => Ok(Frame::Info),
            DAEMON => daemon::request::Daemon::decode(rest)
                .map(Frame::Daemon)
                .map_err(FrameError::Daemon),
            MCP_LIST_TOOLS => Sessioned::<mcp::list_tools::request::Request>::decode(rest)
                .map(Frame::McpListTools)
                .map_err(FrameError::McpParams),
            MCP_LIST_RESOURCES => {
                Sessioned::<mcp::list_resources::request::Request>::decode(rest)
                    .map(Frame::McpListResources)
                    .map_err(FrameError::McpParams)
            }
            MCP_CALL_TOOL => Sessioned::<mcp::call_tool::request::Request>::decode(rest)
                .map(Frame::McpCallTool)
                .map_err(FrameError::McpParams),
            MCP_READ_RESOURCE => {
                Sessioned::<mcp::read_resource::request::Request>::decode(rest)
                    .map(Frame::McpReadResource)
                    .map_err(FrameError::McpParams)
            }
            MCP_NOTIFICATIONS => Sessioned::<mcp::notifications::request::Request>::decode(rest)
                .map(Frame::McpNotifications)
                .map_err(FrameError::McpParams),
            SECRETS => serde_json::from_slice(rest)
                .map(Frame::Secrets)
                .map_err(FrameError::Secrets),
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
    /// The daemon connection's id was not four bytes.
    Daemon(daemon::request::DaemonError),
    /// One of the five MCP exchanges' params did not parse.
    ///
    /// One variant for five tags, because they fail the same way and
    /// the tag already said which was meant.
    McpParams(serde_json::Error),
    /// The secrets' values did not parse as JSON. The message is the
    /// parser's, which quotes no value.
    Secrets(serde_json::Error),
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
            FrameError::Daemon(error) => write!(f, "{error}"),
            FrameError::McpParams(error) => {
                write!(f, "mcp request params did not parse: {error}")
            }
            FrameError::Secrets(error) => {
                write!(f, "secrets feed did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::McpParams(error) | FrameError::Secrets(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Daemon(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
