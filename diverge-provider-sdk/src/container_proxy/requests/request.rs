//! The asks a container can make.

use std::error;
use std::fmt;

use super::FrameError;
use crate::container_proxy::vault;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;

/// One ask, with one byte saying which, and the rest its own
/// payload.
///
/// | kind | ask | payload | answered on |
/// |------|-----|---------|-------------|
/// | `0` | [`McpListTools`](Self::McpListTools) | params JSON | `/mcp/list-tools/{channel}` |
/// | `1` | [`McpListResources`](Self::McpListResources) | params JSON | `/mcp/list-resources/{channel}` |
/// | `2` | [`McpCallTool`](Self::McpCallTool) | params JSON | `/mcp/call-tool/{channel}` |
/// | `3` | [`McpReadResource`](Self::McpReadResource) | params JSON | `/mcp/read-resource/{channel}` |
/// | `4` | [`McpNotifications`](Self::McpNotifications) | none | `/mcp/notifications/{channel}` |
/// | `5` | [`Vault`](Self::Vault) | the operation, as [`vault::Request`] encodes | `/vault/{channel}` |
/// | `6` | [`Command`](Self::Command) | the command, opaque | `/command/{channel}` |
/// | `7` | [`Postgres`](Self::Postgres) | none | `/postgres/{channel}` |
///
/// What each answer path carries is its module's to say: [`mcp`],
/// [`vault`], [`command`](crate::container_proxy::command),
/// [`postgres`](crate::container_proxy::postgres).
#[derive(Debug, Clone, PartialEq)]
pub enum Request<'a> {
    /// What tools are there. [`None`] asks for the first page, which
    /// is also what a caller with nothing to say sends — rmcp makes
    /// the params optional and this keeps that, because params absent
    /// and a cursor absent are different things to a server that
    /// reads them.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources are there. The same shape as
    /// [`McpListTools`](Self::McpListTools), for the same reason: the
    /// same MCP request against a different noun.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one tool: the name and the arguments, as rmcp defines
    /// them. What an argument means belongs to the tool, and nothing
    /// between here and it looks.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one resource, by URI. The URI is the server's to
    /// interpret; a relay that resolved one would be deciding what a
    /// resource is.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything the servers say on their own account, for as long
    /// as the container listens. Carries nothing — in Streamable HTTP
    /// a client opens the notification stream with a bare `GET`, no
    /// method, no body — so the empty payload is not an economy: it
    /// is the request, whole. The server sends nothing on this path
    /// unless the container asked; a container that never asks never
    /// hears one, at no cost to anyone.
    McpNotifications(mcp::notifications::request::Request),
    /// One operation against the caller's vault. See [`vault`].
    Vault(vault::Request<'a>),
    /// A diverge command for the caller to run: bytes in the CLI's
    /// own vocabulary, which this layer never reads. See
    /// [`command`](crate::container_proxy::command).
    Command(&'a [u8]),
    /// A database connection the container's driver just opened,
    /// announced: the server opens `/postgres/{channel}` for it and
    /// the bytes flow there. Carries nothing — pgwire is client-first
    /// and the driver's first bytes wait for the path. See
    /// [`postgres`](crate::container_proxy::postgres).
    Postgres,
}

const MCP_LIST_TOOLS: u8 = 0;
const MCP_LIST_RESOURCES: u8 = 1;
const MCP_CALL_TOOL: u8 = 2;
const MCP_READ_RESOURCE: u8 = 3;
const MCP_NOTIFICATIONS: u8 = 4;
const VAULT: u8 = 5;
const COMMAND: u8 = 6;
const POSTGRES: u8 = 7;

impl Encode for Request<'_> {
    /// The two payloads that can fail: MCP params are JSON, and a
    /// vault key has a length prefix to overflow. A command is bytes
    /// copied, and the notification ask and the announcement are
    /// nothing.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        match self {
            Request::McpListTools(request) => {
                out.extend_from_slice(&[MCP_LIST_TOOLS]);
                request.encode(out).map_err(RequestEncodeError::Mcp)
            }
            Request::McpListResources(request) => {
                out.extend_from_slice(&[MCP_LIST_RESOURCES]);
                request.encode(out).map_err(RequestEncodeError::Mcp)
            }
            Request::McpCallTool(request) => {
                out.extend_from_slice(&[MCP_CALL_TOOL]);
                request.encode(out).map_err(RequestEncodeError::Mcp)
            }
            Request::McpReadResource(request) => {
                out.extend_from_slice(&[MCP_READ_RESOURCE]);
                request.encode(out).map_err(RequestEncodeError::Mcp)
            }
            Request::McpNotifications(request) => {
                out.extend_from_slice(&[MCP_NOTIFICATIONS]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Request::Vault(request) => {
                out.extend_from_slice(&[VAULT]);
                request.encode(out).map_err(RequestEncodeError::Vault)
            }
            Request::Command(command) => {
                out.extend_from_slice(&[COMMAND]);
                out.extend_from_slice(command);
                Ok(())
            }
            Request::Postgres => {
                out.extend_from_slice(&[POSTGRES]);
                Ok(())
            }
        }
    }
}

impl<'a> Request<'a> {
    /// Decode one ask from the bytes after the channel. What it
    /// borrows, it borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (kind, rest) = bytes.split_first().ok_or(FrameError::Truncated)?;
        match *kind {
            MCP_LIST_TOOLS => mcp::list_tools::request::Request::decode(rest)
                .map(Request::McpListTools)
                .map_err(FrameError::Mcp),
            MCP_LIST_RESOURCES => {
                mcp::list_resources::request::Request::decode(rest)
                    .map(Request::McpListResources)
                    .map_err(FrameError::Mcp)
            }
            MCP_CALL_TOOL => mcp::call_tool::request::Request::decode(rest)
                .map(Request::McpCallTool)
                .map_err(FrameError::Mcp),
            MCP_READ_RESOURCE => {
                mcp::read_resource::request::Request::decode(rest)
                    .map(Request::McpReadResource)
                    .map_err(FrameError::Mcp)
            }
            MCP_NOTIFICATIONS => Ok(Request::McpNotifications(
                mcp::notifications::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            VAULT => vault::Request::decode(rest)
                .map(Request::Vault)
                .map_err(FrameError::Vault),
            COMMAND => Ok(Request::Command(rest)),
            POSTGRES => Ok(Request::Postgres),
            other => Err(FrameError::UnknownKind(other)),
        }
    }
}

/// A request that could not be written.
#[derive(Debug)]
pub enum RequestEncodeError {
    /// MCP params that would not serialize.
    Mcp(serde_json::Error),
    /// A vault operation that would not encode.
    Vault(vault::RequestEncodeError),
}

impl fmt::Display for RequestEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestEncodeError::Mcp(error) => {
                write!(f, "mcp params did not serialize: {error}")
            }
            RequestEncodeError::Vault(error) => {
                write!(f, "vault request did not encode: {error}")
            }
        }
    }
}

impl error::Error for RequestEncodeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            RequestEncodeError::Mcp(error) => Some(error),
            RequestEncodeError::Vault(error) => Some(error),
        }
    }
}
