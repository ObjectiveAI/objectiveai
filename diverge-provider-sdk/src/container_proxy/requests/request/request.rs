//! The asks a container can make.

use std::error;
use std::fmt;

use super::FrameError;
use crate::container_proxy::{command, fuse, mcp, postgres, vault};
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};

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
/// | `5` | [`VaultGet`](Self::VaultGet) | `[key…]` | `/vault/get/{channel}` |
/// | `6` | [`VaultSet`](Self::VaultSet) | `[key_len: u16][key…][value…]` | `/vault/set/{channel}` |
/// | `7` | [`VaultDelete`](Self::VaultDelete) | `[key…]` | `/vault/delete/{channel}` |
/// | `8` | [`VaultLock`](Self::VaultLock) | `[ttl: u32][key…]` | `/vault/lock/{channel}` |
/// | `9` | [`VaultUnlock`](Self::VaultUnlock) | `[key…]` | `/vault/unlock/{channel}` |
/// | `10` | [`Command`](Self::Command) | the command, opaque | `/command/{channel}` |
/// | `11` | [`Postgres`](Self::Postgres) | none | `/postgres/{channel}` |
/// | `12` | [`FuseRead`](Self::FuseRead) | `[id_len: u16][id…][path…]` | `/fuse/read/{channel}` |
/// | `13` | [`FuseWrite`](Self::FuseWrite) | `[id_len: u16][id…][path_len: u16][path…][bytes…]` | `/fuse/write/{channel}` |
/// | `14` | [`FuseList`](Self::FuseList) | `[id_len: u16][id…][path…]` | `/fuse/list/{channel}` |
/// | `15` | [`FuseRemove`](Self::FuseRemove) | `[id_len: u16][id…][path…]` | `/fuse/remove/{channel}` |
/// | `16` | [`FuseRename`](Self::FuseRename) | `[id_len: u16][id…][from_len: u16][from…][to…]` | `/fuse/rename/{channel}` |
/// | `17` | [`FuseMkdir`](Self::FuseMkdir) | `[id_len: u16][id…][path…]` | `/fuse/mkdir/{channel}` |
/// | `18` | [`FuseStat`](Self::FuseStat) | `[id_len: u16][id…][path…]` | `/fuse/stat/{channel}` |
///
/// What each answer path carries is its module's to say: [`mcp`],
/// [`vault`], [`command`], [`postgres`], [`fuse`]. Every payload is that path's
/// `request::Request`, so the ask and its answer are found together.
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
    /// Read a key of the caller's vault. See [`vault`].
    VaultGet(vault::get::request::Request<'a>),
    /// Write a key of the caller's vault. See [`vault`].
    VaultSet(vault::set::request::Request<'a>),
    /// Remove a key of the caller's vault. See [`vault`].
    VaultDelete(vault::delete::request::Request<'a>),
    /// Hold a key's lock for a while. See [`vault`].
    VaultLock(vault::lock::request::Request<'a>),
    /// Release a key's lock early. See [`vault`].
    VaultUnlock(vault::unlock::request::Request<'a>),
    /// A diverge command for the caller to run: bytes in the CLI's
    /// own vocabulary, which this layer never reads. See [`command`].
    Command(command::request::Request<'a>),
    /// A database connection the container's driver just opened,
    /// announced: the server opens `/postgres/{channel}` for it and
    /// the bytes flow there. Carries nothing — pgwire is client-first
    /// and the driver's first bytes wait for the path. See [`postgres`].
    Postgres(postgres::request::Request),
    /// Read a file the caller mounted live, by the mount's id and the
    /// file's path in it: the proxy's own ask, for every open of the
    /// file. See [`fuse`].
    FuseRead(fuse::read::request::Request<'a>),
    /// Write such a file, whole: every changed close of it, never on
    /// a read-only mount. See [`fuse`].
    FuseWrite(fuse::write::request::Request<'a>),
    /// List a directory of a mounted tree: every listing in a
    /// directory mount. See [`fuse`].
    FuseList(fuse::list::request::Request<'a>),
    /// Remove a file or an empty directory of a mounted tree. See
    /// [`fuse`].
    FuseRemove(fuse::remove::request::Request<'a>),
    /// Rename an entry within a mounted tree. See [`fuse`].
    FuseRename(fuse::rename::request::Request<'a>),
    /// Make a directory in a mounted tree. See [`fuse`].
    FuseMkdir(fuse::mkdir::request::Request<'a>),
    /// What a mounted entry is and how long: every lookup and every
    /// attribute of a file mount or an entry of a directory mount. See
    /// [`fuse`].
    FuseStat(fuse::stat::request::Request<'a>),
}

const MCP_LIST_TOOLS: u8 = 0;
const MCP_LIST_RESOURCES: u8 = 1;
const MCP_CALL_TOOL: u8 = 2;
const MCP_READ_RESOURCE: u8 = 3;
const MCP_NOTIFICATIONS: u8 = 4;
const VAULT_GET: u8 = 5;
const VAULT_SET: u8 = 6;
const VAULT_DELETE: u8 = 7;
const VAULT_LOCK: u8 = 8;
const VAULT_UNLOCK: u8 = 9;
const COMMAND: u8 = 10;
const POSTGRES: u8 = 11;
const FUSE_READ: u8 = 12;
const FUSE_WRITE: u8 = 13;
const FUSE_LIST: u8 = 14;
const FUSE_REMOVE: u8 = 15;
const FUSE_RENAME: u8 = 16;
const FUSE_MKDIR: u8 = 17;
const FUSE_STAT: u8 = 18;

impl Encode for Request<'_> {
    /// The payloads that can fail: MCP params are JSON, and a vault
    /// `Set`'s key and every fuse ask's id and path have a length
    /// prefix to overflow. Everything else is bytes copied or nothing.
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
            Request::VaultGet(request) => {
                out.extend_from_slice(&[VAULT_GET]);
                request.encode(out).map_err(|error| match error {})
            }
            Request::VaultSet(request) => {
                out.extend_from_slice(&[VAULT_SET]);
                request.encode(out).map_err(RequestEncodeError::Vault)
            }
            Request::VaultDelete(request) => {
                out.extend_from_slice(&[VAULT_DELETE]);
                request.encode(out).map_err(|error| match error {})
            }
            Request::VaultLock(request) => {
                out.extend_from_slice(&[VAULT_LOCK]);
                request.encode(out).map_err(|error| match error {})
            }
            Request::VaultUnlock(request) => {
                out.extend_from_slice(&[VAULT_UNLOCK]);
                request.encode(out).map_err(|error| match error {})
            }
            Request::Command(request) => {
                out.extend_from_slice(&[COMMAND]);
                request.encode(out).map_err(|error| match error {})
            }
            Request::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                request.encode(out).map_err(|error| match error {})
            }
            Request::FuseRead(request) => {
                out.extend_from_slice(&[FUSE_READ]);
                request.encode(out).map_err(RequestEncodeError::Fuse)
            }
            Request::FuseWrite(request) => {
                out.extend_from_slice(&[FUSE_WRITE]);
                request.encode(out).map_err(RequestEncodeError::Fuse)
            }
            Request::FuseList(request) => {
                out.extend_from_slice(&[FUSE_LIST]);
                request.encode(out).map_err(RequestEncodeError::Fuse)
            }
            Request::FuseRemove(request) => {
                out.extend_from_slice(&[FUSE_REMOVE]);
                request.encode(out).map_err(RequestEncodeError::Fuse)
            }
            Request::FuseRename(request) => {
                out.extend_from_slice(&[FUSE_RENAME]);
                request.encode(out).map_err(RequestEncodeError::Fuse)
            }
            Request::FuseMkdir(request) => {
                out.extend_from_slice(&[FUSE_MKDIR]);
                request.encode(out).map_err(RequestEncodeError::Fuse)
            }
            Request::FuseStat(request) => {
                out.extend_from_slice(&[FUSE_STAT]);
                request.encode(out).map_err(RequestEncodeError::Fuse)
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
            VAULT_GET => vault::get::request::Request::decode(rest)
                .map(Request::VaultGet)
                .map_err(FrameError::Vault),
            VAULT_SET => vault::set::request::Request::decode(rest)
                .map(Request::VaultSet)
                .map_err(FrameError::Vault),
            VAULT_DELETE => vault::delete::request::Request::decode(rest)
                .map(Request::VaultDelete)
                .map_err(FrameError::Vault),
            VAULT_LOCK => vault::lock::request::Request::decode(rest)
                .map(Request::VaultLock)
                .map_err(FrameError::Vault),
            VAULT_UNLOCK => vault::unlock::request::Request::decode(rest)
                .map(Request::VaultUnlock)
                .map_err(FrameError::Vault),
            COMMAND => Ok(Request::Command(command::request::Request(rest))),
            POSTGRES => Ok(Request::Postgres(
                postgres::request::Request::decode(rest)
                    .unwrap_or_else(|error| match error {}),
            )),
            FUSE_READ => fuse::read::request::Request::decode(rest)
                .map(Request::FuseRead)
                .map_err(FrameError::Fuse),
            FUSE_WRITE => fuse::write::request::Request::decode(rest)
                .map(Request::FuseWrite)
                .map_err(FrameError::Fuse),
            FUSE_LIST => fuse::list::request::Request::decode(rest)
                .map(Request::FuseList)
                .map_err(FrameError::Fuse),
            FUSE_REMOVE => fuse::remove::request::Request::decode(rest)
                .map(Request::FuseRemove)
                .map_err(FrameError::Fuse),
            FUSE_RENAME => fuse::rename::request::Request::decode(rest)
                .map(Request::FuseRename)
                .map_err(FrameError::Fuse),
            FUSE_MKDIR => fuse::mkdir::request::Request::decode(rest)
                .map(Request::FuseMkdir)
                .map_err(FrameError::Fuse),
            FUSE_STAT => fuse::stat::request::Request::decode(rest)
                .map(Request::FuseStat)
                .map_err(FrameError::Fuse),
            other => Err(FrameError::UnknownKind(other)),
        }
    }
}

/// A request that could not be written.
#[derive(Debug)]
pub enum RequestEncodeError {
    /// MCP params that would not serialize.
    Mcp(serde_json::Error),
    /// A vault `Set` whose key would not fit its length prefix.
    Vault(vault::RequestEncodeError),
    /// A fuse ask whose id or path would not fit its length prefix.
    Fuse(fuse::RequestEncodeError),
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
            RequestEncodeError::Fuse(error) => {
                write!(f, "fuse request did not encode: {error}")
            }
        }
    }
}

impl error::Error for RequestEncodeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            RequestEncodeError::Mcp(error) => Some(error),
            RequestEncodeError::Vault(error) => Some(error),
            RequestEncodeError::Fuse(error) => Some(error),
        }
    }
}
