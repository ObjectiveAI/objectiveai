//! What a server's channel request frame carries for an agent container run.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::{
    authorize, command, fetch_directory, fetch_file, postgres, vault,
    write_bytes,
};
use crate::shared::{mcp, oci};

/// What a provider asks a caller for while an agent container runs.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Oci`](Self::Oci) |
/// | `1` | [`Authorize`](Self::Authorize) |
/// | `2` | [`Write`](Self::Write) |
/// | `3` | [`FetchFile`](Self::FetchFile) |
/// | `4` | [`FetchDirectory`](Self::FetchDirectory) |
/// | `5` | [`Postgres`](Self::Postgres) |
/// | `6` | [`Command`](Self::Command) |
/// | `7` | [`VaultGet`](Self::VaultGet) |
/// | `8` | [`VaultSet`](Self::VaultSet) |
/// | `9` | [`VaultDelete`](Self::VaultDelete) |
/// | `10` | [`VaultLock`](Self::VaultLock) |
/// | `11` | [`VaultUnlock`](Self::VaultUnlock) |
/// | `12` | [`McpListTools`](Self::McpListTools) |
/// | `13` | [`McpListResources`](Self::McpListResources) |
/// | `14` | [`McpCallTool`](Self::McpCallTool) |
/// | `15` | [`McpReadResource`](Self::McpReadResource) |
/// | `16` | [`McpNotifications`](Self::McpNotifications) |
///
/// The same seventeen in both families, in the same order. The first
/// five are the provider's own asks — an image the caller serves, a
/// connector's authorization, a write's content, mounted content it
/// does not hold — and the rest are the CONTAINER's, relayed: its
/// database connections, its commands, its vault, and its tool calls
/// outward to the caller's MCP servers. A connector's scope has none
/// of these but [`Write`](Self::Write); the container's asks go to
/// whoever runs it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One request against the caller's registry. Tag `0`.
    ///
    /// Opened only for an
    /// [`Image::Client`](crate::shared::containers::request::Image::Client)
    /// container, and opened by the container RUNTIME's appetite rather
    /// than the provider's: the provider serves a registry endpoint,
    /// the runtime pulls from it, and every request the runtime makes
    /// that the provider cannot answer from what it holds becomes one
    /// of these. The provider understands none of it — see
    /// [`oci`](crate::shared::oci).
    Oci(oci::request::Request<'a>),
    /// Ask the caller whether a connector may attach. Tag `1`.
    ///
    /// Opened when one arrives; see
    /// [`authorize`](crate::shared::containers::authorize).
    Authorize(authorize::request::Authorize),
    /// Send the content for a write. Tag `2`.
    ///
    /// Opened in answer to a write the caller started. A write cannot
    /// carry its own content — only a responder can finish a channel —
    /// so the bytes travel as responses on this one. See
    /// [`write_bytes`](crate::shared::containers::write_bytes).
    Write(write_bytes::request::Request),
    /// Send a mounted file the provider does not hold. Tag `3`.
    ///
    /// See [`fetch_file`](crate::shared::containers::fetch_file).
    FetchFile(fetch_file::request::Request),
    /// Send a mounted directory the provider does not hold. Tag `4`.
    ///
    /// See [`fetch_directory`](crate::shared::containers::fetch_directory).
    FetchDirectory(fetch_directory::request::Request),
    /// The provider's half of a database connection the container
    /// opened. Tag `5`.
    ///
    /// What comes back is everything the database says; the caller
    /// opens the other half, or declines. See
    /// [`postgres`](crate::shared::containers::postgres).
    Postgres(postgres::request::Postgres),
    /// Run a command the container asked for. Tag `6`.
    ///
    /// Opaque bytes in the CLI's vocabulary; the items come back one
    /// per frame. See [`command`](crate::shared::containers::command).
    Command(command::request::Request<'a>),
    /// Read a vault key. Tag `7`.
    VaultGet(vault::get::request::Request<'a>),
    /// Write a vault key. Tag `8`.
    VaultSet(vault::set::request::Request<'a>),
    /// Remove a vault key. Tag `9`.
    VaultDelete(vault::delete::request::Request<'a>),
    /// Hold a vault key's lock. Tag `10`.
    VaultLock(vault::lock::request::Request<'a>),
    /// Release a vault key's lock. Tag `11`.
    ///
    /// The five vault asks are the container's; see
    /// [`vault`](crate::shared::containers::vault) for what each
    /// carries and how a lock behaves.
    VaultUnlock(vault::unlock::request::Request<'a>),
    /// What tools the caller's servers have. Tag `12`.
    ///
    /// The container asking OUTWARD: an agent's tool calls go to
    /// servers that live with the caller, so the five exchanges in
    /// [`shared::mcp`](crate::shared::mcp) travel this direction too.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources they have. Tag `13`.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one of their tools. Tag `14`.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one of their resources. Tag `15`.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything they say on their own account. Tag `16`.
    McpNotifications(mcp::notifications::request::Request),
}

/// Tag for [`Frame::Oci`].
const OCI: u8 = 0;

/// Tag for [`Frame::Authorize`].
const AUTHORIZE: u8 = 1;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 2;

/// Tag for [`Frame::FetchFile`].
const FETCH_FILE: u8 = 3;

/// Tag for [`Frame::FetchDirectory`].
const FETCH_DIRECTORY: u8 = 4;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 5;

/// Tag for [`Frame::Command`].
const COMMAND: u8 = 6;

/// Tag for [`Frame::VaultGet`].
const VAULT_GET: u8 = 7;

/// Tag for [`Frame::VaultSet`].
const VAULT_SET: u8 = 8;

/// Tag for [`Frame::VaultDelete`].
const VAULT_DELETE: u8 = 9;

/// Tag for [`Frame::VaultLock`].
const VAULT_LOCK: u8 = 10;

/// Tag for [`Frame::VaultUnlock`].
const VAULT_UNLOCK: u8 = 11;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 12;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 13;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 14;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 15;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 16;

impl Encode for Frame<'_> {
    /// The JSON failure from the asks that are JSON, or a vault key
    /// too long for its prefix; everything else is bytes copied.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Oci(request) => {
                out.extend_from_slice(&[OCI]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Authorize(authorize) => {
                out.extend_from_slice(&[AUTHORIZE]);
                serde_json::to_writer(out, authorize)
                    .map_err(FrameEncodeError::Json)
            }
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::FetchFile(request) => {
                out.extend_from_slice(&[FETCH_FILE]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::FetchDirectory(request) => {
                out.extend_from_slice(&[FETCH_DIRECTORY]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
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

/// An agent container run channel request that could not be written.
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
            OCI => Ok(Frame::Oci(
                oci::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            AUTHORIZE => serde_json::from_slice(rest)
                .map(Frame::Authorize)
                .map_err(FrameError::Authorize),
            WRITE => write_bytes::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
            FETCH_FILE => fetch_file::request::Request::decode(rest)
                .map(Frame::FetchFile)
                .map_err(FrameError::Fetch),
            FETCH_DIRECTORY => fetch_directory::request::Request::decode(rest)
                .map(Frame::FetchDirectory)
                .map_err(FrameError::Fetch),
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

/// An agent container run channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's seventeen.
    UnknownTag(u8),
    /// The authorization request did not parse.
    Authorize(serde_json::Error),
    /// The write content request did not decode.
    Write(write_bytes::request::RequestError),
    /// A fetch request did not parse.
    Fetch(serde_json::Error),
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
                f.write_str("agents run channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agents run channel request tag {tag}")
            }
            FrameError::Authorize(error) => {
                write!(f, "authorization request did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write content request did not decode: {error}")
            }
            FrameError::Fetch(error) => {
                write!(f, "fetch request did not parse: {error}")
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
            FrameError::Authorize(error)
            | FrameError::Fetch(error)
            | FrameError::McpParams(error) => Some(error),
            FrameError::Write(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Vault(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
