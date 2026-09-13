//! What a server's channel request frame carries for a tool
//! container begin.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::{command, postgres, vault};
use crate::shared::mcp;

/// What the proxy asks the server for once a tool container
/// has begun.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Postgres`](Self::Postgres) |
/// | `1` | [`Command`](Self::Command) |
/// | `2` | [`VaultGet`](Self::VaultGet) |
/// | `3` | [`VaultSet`](Self::VaultSet) |
/// | `4` | [`VaultDelete`](Self::VaultDelete) |
/// | `5` | [`VaultLock`](Self::VaultLock) |
/// | `6` | [`VaultUnlock`](Self::VaultUnlock) |
/// | `7` | [`McpListTools`](Self::McpListTools) |
/// | `8` | [`McpListResources`](Self::McpListResources) |
/// | `9` | [`McpCallTool`](Self::McpCallTool) |
/// | `10` | [`McpReadResource`](Self::McpReadResource) |
/// | `11` | [`McpNotifications`](Self::McpNotifications) |
///
/// The same twelve in both families, in the same order: everything a
/// container asks of the world outside — its database connections,
/// its commands, its vault, its tool calls outward to the caller's
/// MCP servers — which the server relays to the caller as the
/// provider protocol's own channel requests. The fuse asks are not
/// here; each rides the scope of the mount it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// The proxy's half of a database connection the container's
    /// driver opened. Tag `0`.
    ///
    /// An id the proxy minted. What comes back is everything the
    /// database says; the server opens the other half, or declines by
    /// finishing this one with nothing before it. See
    /// [`postgres`](crate::shared::containers::postgres).
    Postgres(postgres::request::Postgres),
    /// Run a command the container asked for. Tag `1`.
    ///
    /// Opaque bytes in the CLI's vocabulary; the items come back one
    /// per frame. See [`command`](crate::shared::containers::command).
    Command(command::request::Request<'a>),
    /// Read a vault key. Tag `2`.
    VaultGet(vault::get::request::Request<'a>),
    /// Write a vault key. Tag `3`.
    VaultSet(vault::set::request::Request<'a>),
    /// Remove a vault key. Tag `4`.
    VaultDelete(vault::delete::request::Request<'a>),
    /// Hold a vault key's lock. Tag `5`.
    VaultLock(vault::lock::request::Request<'a>),
    /// Release a vault key's lock. Tag `6`.
    ///
    /// The five vault asks are the container's; see
    /// [`vault`](crate::shared::containers::vault) for what each
    /// carries and how a lock behaves.
    VaultUnlock(vault::unlock::request::Request<'a>),
    /// What tools the caller's servers have. Tag `7`.
    ///
    /// The container asking OUTWARD: an agent's tool calls go to
    /// servers that live with the caller, so the five exchanges in
    /// [`shared::mcp`](crate::shared::mcp) travel this direction too.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources they have. Tag `8`.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one of their tools. Tag `9`.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one of their resources. Tag `10`.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything they say on their own account. Tag `11`.
    McpNotifications(mcp::notifications::request::Request),
}

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 0;

/// Tag for [`Frame::Command`].
const COMMAND: u8 = 1;

/// Tag for [`Frame::VaultGet`].
const VAULT_GET: u8 = 2;

/// Tag for [`Frame::VaultSet`].
const VAULT_SET: u8 = 3;

/// Tag for [`Frame::VaultDelete`].
const VAULT_DELETE: u8 = 4;

/// Tag for [`Frame::VaultLock`].
const VAULT_LOCK: u8 = 5;

/// Tag for [`Frame::VaultUnlock`].
const VAULT_UNLOCK: u8 = 6;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 7;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 8;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 9;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 10;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 11;

impl Encode for Frame<'_> {
    /// The JSON failure from the asks that are JSON, or a vault key
    /// too long for its prefix; everything else is bytes copied.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Command(request) => {
                out.extend_from_slice(&[COMMAND]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::VaultGet(request) => {
                out.extend_from_slice(&[VAULT_GET]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::VaultSet(request) => {
                out.extend_from_slice(&[VAULT_SET]);
                request.encode(out).map_err(FrameEncodeError::Vault)
            }
            Frame::VaultDelete(request) => {
                out.extend_from_slice(&[VAULT_DELETE]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::VaultLock(request) => {
                out.extend_from_slice(&[VAULT_LOCK]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::VaultUnlock(request) => {
                out.extend_from_slice(&[VAULT_UNLOCK]);
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

/// A tools begin channel request that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// A JSON ask did not serialize.
    Json(serde_json::Error),
    /// A vault ask would not encode: a key too long for its prefix.
    Vault(vault::RequestEncodeError),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Json(error) => {
                write!(f, "channel request did not serialize: {error}")
            }
            FrameEncodeError::Vault(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Json(error) => Some(error),
            FrameEncodeError::Vault(error) => Some(error),
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Several ways to fail, and each names which ask it was.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            COMMAND => Ok(Frame::Command(
                command::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            VAULT_GET => vault::get::request::Request::decode(rest)
                .map(Frame::VaultGet)
                .map_err(FrameError::Vault),
            VAULT_SET => vault::set::request::Request::decode(rest)
                .map(Frame::VaultSet)
                .map_err(FrameError::Vault),
            VAULT_DELETE => vault::delete::request::Request::decode(rest)
                .map(Frame::VaultDelete)
                .map_err(FrameError::Vault),
            VAULT_LOCK => vault::lock::request::Request::decode(rest)
                .map(Frame::VaultLock)
                .map_err(FrameError::Vault),
            VAULT_UNLOCK => vault::unlock::request::Request::decode(rest)
                .map(Frame::VaultUnlock)
                .map_err(FrameError::Vault),
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
    /// A tag that is none of this frame's twelve.
    UnknownTag(u8),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// A vault ask did not decode.
    Vault(vault::RequestError),
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
            FrameError::Vault(error) => write!(f, "{error}"),
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
            FrameError::Vault(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
