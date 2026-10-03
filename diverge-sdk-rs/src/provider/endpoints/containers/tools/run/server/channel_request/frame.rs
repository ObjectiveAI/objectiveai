//! What a server's channel request frame carries for a tool container run.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::{authorize, command, fuse, oci, postgres, tools, vault, write_bytes};
use crate::shared::mcp;

/// What a provider asks a caller for while a tool container runs.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`OciManifest`](Self::OciManifest) |
/// | `1` | [`OciBlob`](Self::OciBlob) |
/// | `2` | [`OciHas`](Self::OciHas) |
/// | `3` | [`AuthorizeConnect`](Self::AuthorizeConnect) |
/// | `4` | [`AuthorizeList`](Self::AuthorizeList) |
/// | `5` | [`Tools`](Self::Tools) |
/// | `6` | [`Write`](Self::Write) |
/// | `7` | [`Postgres`](Self::Postgres) |
/// | `8` | [`Command`](Self::Command) |
/// | `9` | [`VaultGet`](Self::VaultGet) |
/// | `10` | [`VaultSet`](Self::VaultSet) |
/// | `11` | [`VaultDelete`](Self::VaultDelete) |
/// | `12` | [`VaultLock`](Self::VaultLock) |
/// | `13` | [`VaultUnlock`](Self::VaultUnlock) |
/// | `14` | [`McpListTools`](Self::McpListTools) |
/// | `15` | [`McpListResources`](Self::McpListResources) |
/// | `16` | [`McpCallTool`](Self::McpCallTool) |
/// | `17` | [`McpReadResource`](Self::McpReadResource) |
/// | `18` | [`McpNotifications`](Self::McpNotifications) |
/// | `19` | [`FuseRead`](Self::FuseRead) |
/// | `20` | [`FuseWrite`](Self::FuseWrite) |
/// | `21` | [`FuseList`](Self::FuseList) |
/// | `22` | [`FuseRemove`](Self::FuseRemove) |
/// | `23` | [`FuseRename`](Self::FuseRename) |
/// | `24` | [`FuseMkdir`](Self::FuseMkdir) |
/// | `25` | [`FuseStat`](Self::FuseStat) |
/// | `26` | [`FuseTruncate`](Self::FuseTruncate) |
/// | `27` | [`FuseSetattr`](Self::FuseSetattr) |
///
/// The same twenty-eight in both families, in the same order. The
/// first seven are the provider's own asks — whether the caller holds
/// an image, its manifest and blobs, a connector's authorization, a
/// lister's, the tools the container declared, a write's content — and the rest
/// are the CONTAINER's, relayed: its database
/// connections, its commands, its vault, its tool calls outward to the
/// caller's MCP servers, and the files the caller mounted live. A
/// connector's scope has none of these but [`Write`](Self::Write); the container's asks go to
/// whoever runs it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// A manifest of an image the caller holds, by digest. Tag `0`.
    ///
    /// Opened when the provider takes the image from the caller, and
    /// opened by the container RUNTIME's appetite rather
    /// than the provider's: the provider's registry serves the pull,
    /// and a manifest its store does not hold becomes one of these.
    /// See [`oci`](crate::shared::containers::oci).
    OciManifest(oci::manifest::request::Request),
    /// A blob of an image the caller holds, by digest. Tag `1`.
    ///
    /// The other half of a pull; see
    /// [`oci`](crate::shared::containers::oci).
    OciBlob(oci::blob::request::Request),
    /// Whether the caller holds an image, by name and digest. Tag `2`.
    ///
    /// Opened by a provider that would take the image from the caller,
    /// before it asks for anything of it; see
    /// [`oci`](crate::shared::containers::oci).
    OciHas(oci::has::request::Request),
    /// Ask the caller whether a connector may attach. Tag `3`.
    ///
    /// Opened when one arrives; see
    /// [`authorize`](crate::shared::containers::authorize).
    AuthorizeConnect(authorize::request::AuthorizeConnect),
    /// Ask the caller whether a lister may see this container: who
    /// asks, attested. Tag `4`.
    AuthorizeList(authorize::request::AuthorizeList),
    /// Deploy the tools the container declared. Tag `5`.
    ///
    /// Opened once, after the proxy's `Begun` carried a non-empty
    /// list and before the id; never on a connect. See
    /// [`tools`](crate::shared::containers::tools).
    Tools(tools::request::Request<'a>),
    /// Send the content for a write. Tag `6`.
    ///
    /// Opened in answer to a write the caller started. A write cannot
    /// carry its own content — only a responder can finish a channel —
    /// so the bytes travel as responses on this one. See
    /// [`write_bytes`](crate::shared::containers::write_bytes).
    Write(write_bytes::request::Request),
    /// The provider's half of a database connection the container
    /// opened. Tag `7`.
    ///
    /// What comes back is everything the database says; the caller
    /// opens the other half, or declines. See
    /// [`postgres`](crate::shared::containers::postgres).
    Postgres(postgres::request::Postgres),
    /// Run a command the container asked for. Tag `8`.
    ///
    /// Opaque bytes in the CLI's vocabulary; the items come back one
    /// per frame. See [`command`](crate::shared::containers::command).
    Command(command::request::Request<'a>),
    /// Read a vault key. Tag `9`.
    VaultGet(vault::get::request::Request<'a>),
    /// Write a vault key. Tag `10`.
    VaultSet(vault::set::request::Request<'a>),
    /// Remove a vault key. Tag `11`.
    VaultDelete(vault::delete::request::Request<'a>),
    /// Hold a vault key's lock. Tag `12`.
    VaultLock(vault::lock::request::Request<'a>),
    /// Release a vault key's lock. Tag `13`.
    ///
    /// The five vault asks are the container's; see
    /// [`vault`](crate::shared::containers::vault) for what each
    /// carries and how a lock behaves.
    VaultUnlock(vault::unlock::request::Request<'a>),
    /// What tools the caller's servers have. Tag `14`.
    ///
    /// The container asking OUTWARD: an agent's tool calls go to
    /// servers that live with the caller, so the five exchanges in
    /// [`shared::mcp`](crate::shared::mcp) travel this direction too.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources they have. Tag `15`.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one of their tools. Tag `16`.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one of their resources. Tag `17`.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything they say on their own account. Tag `18`.
    McpNotifications(mcp::notifications::request::Request),
    /// Read a piece of a file the caller mounted live, by the mount's
    /// id, the file's path in it, an offset and a length. Tag `19`.
    ///
    /// The container's proxy asking on behalf of a FUSE mount: every
    /// `read(2)` of the file. See
    /// [`fuse`](crate::shared::containers::fuse).
    FuseRead(fuse::read::request::Request<'a>),
    /// Write a piece of a file the caller mounted live, in place at
    /// an offset. Tag `20`.
    ///
    /// Every `write(2)` of the file.
    FuseWrite(fuse::write::request::Request<'a>),
    /// List a directory of a tree the caller mounted live. Tag `21`.
    ///
    /// Every listing in a directory mount: names and kinds.
    FuseList(fuse::list::request::Request<'a>),
    /// Remove a file or an empty directory of such a tree. Tag `22`.
    FuseRemove(fuse::remove::request::Request<'a>),
    /// Rename an entry within such a tree. Tag `23`.
    FuseRename(fuse::rename::request::Request<'a>),
    /// Make a directory in such a tree. Tag `24`.
    FuseMkdir(fuse::mkdir::request::Request<'a>),
    /// What an entry the caller mounted live is: kind, size, mode,
    /// owner, group and times. Tag `25`.
    ///
    /// Every attribute of a file mount, and every lookup and
    /// attribute of an entry in a directory mount: a `stat` costs
    /// fifty-seven bytes back, not the file.
    FuseStat(fuse::stat::request::Request<'a>),
    /// Set a file the caller mounted live to a length. Tag `26`.
    ///
    /// Every `truncate(2)`, `ftruncate(2)` and `O_TRUNC` open.
    FuseTruncate(fuse::truncate::request::Request<'a>),
    /// Set some attributes of an entry the caller mounted live. Tag
    /// `26`.
    ///
    /// Every `chmod(2)`, `chown(2)` and `utimensat(2)`.
    FuseSetattr(fuse::setattr::request::Request<'a>),
}

/// Tag for [`Frame::OciManifest`].
const OCI_MANIFEST: u8 = 0;

/// Tag for [`Frame::OciBlob`].
const OCI_BLOB: u8 = 1;

/// Tag for [`Frame::OciHas`].
const OCI_HAS: u8 = 2;

/// Tag for [`Frame::AuthorizeConnect`].
const AUTHORIZE_CONNECT: u8 = 3;

/// Tag for [`Frame::AuthorizeList`].
const AUTHORIZE_LIST: u8 = 4;

/// Tag for [`Frame::Tools`].
const TOOLS: u8 = 5;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 6;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 7;

/// Tag for [`Frame::Command`].
const COMMAND: u8 = 8;

/// Tag for [`Frame::VaultGet`].
const VAULT_GET: u8 = 9;

/// Tag for [`Frame::VaultSet`].
const VAULT_SET: u8 = 10;

/// Tag for [`Frame::VaultDelete`].
const VAULT_DELETE: u8 = 11;

/// Tag for [`Frame::VaultLock`].
const VAULT_LOCK: u8 = 12;

/// Tag for [`Frame::VaultUnlock`].
const VAULT_UNLOCK: u8 = 13;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 14;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 15;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 16;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 17;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 18;

/// Tag for [`Frame::FuseRead`].
const FUSE_READ: u8 = 19;

/// Tag for [`Frame::FuseWrite`].
const FUSE_WRITE: u8 = 20;

/// Tag for [`Frame::FuseList`].
const FUSE_LIST: u8 = 21;

/// Tag for [`Frame::FuseRemove`].
const FUSE_REMOVE: u8 = 22;

/// Tag for [`Frame::FuseRename`].
const FUSE_RENAME: u8 = 23;

/// Tag for [`Frame::FuseMkdir`].
const FUSE_MKDIR: u8 = 24;

/// Tag for [`Frame::FuseStat`].
const FUSE_STAT: u8 = 25;

/// Tag for [`Frame::FuseTruncate`].
const FUSE_TRUNCATE: u8 = 26;

/// Tag for [`Frame::FuseSetattr`].
const FUSE_SETATTR: u8 = 27;

impl Encode for Frame<'_> {
    /// The JSON failure from the asks that are JSON, or a vault key
    /// or a fuse id or path too long for its prefix; everything else
    /// is bytes copied.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::OciManifest(request) => {
                out.extend_from_slice(&[OCI_MANIFEST]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::OciBlob(request) => {
                out.extend_from_slice(&[OCI_BLOB]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::OciHas(request) => {
                out.extend_from_slice(&[OCI_HAS]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::AuthorizeConnect(authorize) => {
                out.extend_from_slice(&[AUTHORIZE_CONNECT]);
                serde_json::to_writer(out, authorize)
                    .map_err(FrameEncodeError::Json)
            }
            Frame::AuthorizeList(list) => {
                out.extend_from_slice(&[AUTHORIZE_LIST]);
                serde_json::to_writer(out, list)
                    .map_err(FrameEncodeError::Json)
            }
            Frame::Tools(request) => {
                out.extend_from_slice(&[TOOLS]);
                request.encode(out).map_err(FrameEncodeError::Json)
            }
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
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
            Frame::FuseRead(request) => {
                out.extend_from_slice(&[FUSE_READ]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
            Frame::FuseWrite(request) => {
                out.extend_from_slice(&[FUSE_WRITE]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
            Frame::FuseList(request) => {
                out.extend_from_slice(&[FUSE_LIST]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
            Frame::FuseRemove(request) => {
                out.extend_from_slice(&[FUSE_REMOVE]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
            Frame::FuseRename(request) => {
                out.extend_from_slice(&[FUSE_RENAME]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
            Frame::FuseMkdir(request) => {
                out.extend_from_slice(&[FUSE_MKDIR]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
            Frame::FuseStat(request) => {
                out.extend_from_slice(&[FUSE_STAT]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
            Frame::FuseTruncate(request) => {
                out.extend_from_slice(&[FUSE_TRUNCATE]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
            Frame::FuseSetattr(request) => {
                out.extend_from_slice(&[FUSE_SETATTR]);
                request.encode(out).map_err(FrameEncodeError::Fuse)
            }
        }
    }
}

/// A tool container run channel request that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// A JSON ask did not serialize.
    Json(serde_json::Error),
    /// A vault ask would not encode: a key too long for its prefix.
    Vault(vault::RequestEncodeError),
    /// A fuse ask would not encode: an id or a path too long for its
    /// prefix.
    Fuse(fuse::RequestEncodeError),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Json(error) => {
                write!(f, "channel request did not serialize: {error}")
            }
            FrameEncodeError::Vault(error) => write!(f, "{error}"),
            FrameEncodeError::Fuse(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Json(error) => Some(error),
            FrameEncodeError::Vault(error) => Some(error),
            FrameEncodeError::Fuse(error) => Some(error),
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Several ways to fail, and each names which ask it was.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OCI_MANIFEST => oci::manifest::request::Request::decode(rest)
                .map(Frame::OciManifest)
                .map_err(FrameError::Oci),
            OCI_BLOB => oci::blob::request::Request::decode(rest)
                .map(Frame::OciBlob)
                .map_err(FrameError::Oci),
            OCI_HAS => oci::has::request::Request::decode(rest)
                .map(Frame::OciHas)
                .map_err(FrameError::Oci),
            AUTHORIZE_CONNECT => serde_json::from_slice(rest)
                .map(Frame::AuthorizeConnect)
                .map_err(FrameError::AuthorizeConnect),
            AUTHORIZE_LIST => serde_json::from_slice(rest)
                .map(Frame::AuthorizeList)
                .map_err(FrameError::AuthorizeList),
            TOOLS => tools::request::Request::decode(rest)
                .map(Frame::Tools)
                .map_err(FrameError::Tools),
            WRITE => write_bytes::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
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
            FUSE_READ => fuse::read::request::Request::decode(rest)
                .map(Frame::FuseRead)
                .map_err(FrameError::Fuse),
            FUSE_WRITE => fuse::write::request::Request::decode(rest)
                .map(Frame::FuseWrite)
                .map_err(FrameError::Fuse),
            FUSE_LIST => fuse::list::request::Request::decode(rest)
                .map(Frame::FuseList)
                .map_err(FrameError::Fuse),
            FUSE_REMOVE => fuse::remove::request::Request::decode(rest)
                .map(Frame::FuseRemove)
                .map_err(FrameError::Fuse),
            FUSE_RENAME => fuse::rename::request::Request::decode(rest)
                .map(Frame::FuseRename)
                .map_err(FrameError::Fuse),
            FUSE_MKDIR => fuse::mkdir::request::Request::decode(rest)
                .map(Frame::FuseMkdir)
                .map_err(FrameError::Fuse),
            FUSE_STAT => fuse::stat::request::Request::decode(rest)
                .map(Frame::FuseStat)
                .map_err(FrameError::Fuse),
            FUSE_TRUNCATE => fuse::truncate::request::Request::decode(rest)
                .map(Frame::FuseTruncate)
                .map_err(FrameError::Fuse),
            FUSE_SETATTR => fuse::setattr::request::Request::decode(rest)
                .map(Frame::FuseSetattr)
                .map_err(FrameError::Fuse),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tool container run channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's twenty-eight.
    UnknownTag(u8),
    /// An image ask did not parse.
    Oci(serde_json::Error),
    /// The authorization request did not parse.
    AuthorizeConnect(serde_json::Error),
    /// The listing authorization request did not parse.
    AuthorizeList(serde_json::Error),
    /// The tools did not parse.
    Tools(serde_json::Error),
    /// The write content request did not decode.
    Write(write_bytes::request::RequestError),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// A vault ask did not decode.
    Vault(vault::RequestError),
    /// A fuse ask did not decode.
    Fuse(fuse::RequestError),
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
                f.write_str("tools run channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools run channel request tag {tag}")
            }
            FrameError::Oci(error) => {
                write!(f, "image ask did not parse: {error}")
            }
            FrameError::AuthorizeConnect(error) => {
                write!(f, "authorization request did not parse: {error}")
            }
            FrameError::AuthorizeList(error) => {
                write!(f, "listing authorization request did not parse: {error}")
            }
            FrameError::Tools(error) => {
                write!(f, "tools did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write content request did not decode: {error}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::Vault(error) => write!(f, "{error}"),
            FrameError::Fuse(error) => write!(f, "{error}"),
            FrameError::McpParams(error) => {
                write!(f, "mcp request params did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Oci(error)
            | FrameError::AuthorizeConnect(error)
            | FrameError::AuthorizeList(error)
            | FrameError::Tools(error)
            | FrameError::McpParams(error) => Some(error),
            FrameError::Write(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Vault(error) => Some(error),
            FrameError::Fuse(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
