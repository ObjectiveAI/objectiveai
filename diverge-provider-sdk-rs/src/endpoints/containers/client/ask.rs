//! One ask a run scope's provider makes, owned.

use bytes::Bytes;
use rmcp::model::{CallToolRequestParams, PaginatedRequestParams, ReadResourceRequestParams};

use crate::endpoints::containers::{agents, tools};
use crate::shared::containers::authorize;

/// A server-opened channel request on a run scope, with nothing
/// borrowed: what the serving loop hands to a task.
///
/// The two families' `server::channel_request::Frame`s carry the same
/// twenty-four asks in the same order with the same payloads, and
/// borrow from the frame they were decoded from; this is the one
/// owned form both convert into, so the answer to each is written
/// once. Which family it came from does not matter to the answer: the
/// caller's stores, vault, servers and dialer are the same behind
/// either.
#[derive(Debug, Clone)]
pub enum Ask {
    /// The manifest of an image the caller holds, by digest.
    OciManifest(String),
    /// A blob of an image the caller holds, by digest.
    OciBlob(String),
    /// Whether the caller holds an image: its name, its digest.
    OciHas(String, String),
    /// Whether a connector may attach.
    Authorize(authorize::request::Authorize),
    /// The content of a write this caller started, by its id.
    Write(u32),
    /// A database connection the container opened, by the id the
    /// provider minted.
    Postgres(u32),
    /// A command, opaque.
    Command(Bytes),
    /// A vault key's value.
    VaultGet(String),
    /// A vault key set.
    VaultSet(String, Bytes),
    /// A vault key removed.
    VaultDelete(String),
    /// A vault key's lock, for this many seconds.
    VaultLock(String, u32),
    /// A vault key's lock released.
    VaultUnlock(String),
    /// `tools/list`.
    McpListTools(Option<PaginatedRequestParams>),
    /// `resources/list`.
    McpListResources(Option<PaginatedRequestParams>),
    /// `tools/call`.
    McpCallTool(CallToolRequestParams),
    /// `resources/read`.
    McpReadResource(ReadResourceRequestParams),
    /// The servers' notifications, for as long as the channel lives.
    McpNotifications,
    /// A mounted file read: the mount's id, the path.
    FuseRead(String, String),
    /// A mounted file written whole: the id, the path, the bytes.
    FuseWrite(String, String, Bytes),
    /// A mounted directory listed: the id, the path.
    FuseList(String, String),
    /// A mounted entry removed: the id, the path.
    FuseRemove(String, String),
    /// A mounted entry renamed: the id, from, to.
    FuseRename(String, String, String),
    /// A mounted directory made: the id, the path.
    FuseMkdir(String, String),
    /// A mounted entry described: the id, the path.
    FuseStat(String, String),
}

impl From<agents::run::server::channel_request::Frame<'_>> for Ask {
    fn from(frame: agents::run::server::channel_request::Frame<'_>) -> Self {
        use agents::run::server::channel_request::Frame;
        match frame {
            Frame::OciManifest(request) => Ask::OciManifest(request.digest),
            Frame::OciBlob(request) => Ask::OciBlob(request.digest),
            Frame::OciHas(request) => Ask::OciHas(request.name, request.digest),
            Frame::Authorize(request) => Ask::Authorize(request),
            Frame::Write(request) => Ask::Write(request.write_id),
            Frame::Postgres(request) => Ask::Postgres(request.connection_id),
            Frame::Command(request) => Ask::Command(Bytes::copy_from_slice(request.0)),
            Frame::VaultGet(request) => Ask::VaultGet(request.key.to_string()),
            Frame::VaultSet(request) => {
                Ask::VaultSet(request.key.to_string(), Bytes::copy_from_slice(request.value))
            }
            Frame::VaultDelete(request) => Ask::VaultDelete(request.key.to_string()),
            Frame::VaultLock(request) => Ask::VaultLock(request.key.to_string(), request.ttl),
            Frame::VaultUnlock(request) => Ask::VaultUnlock(request.key.to_string()),
            Frame::McpListTools(request) => Ask::McpListTools(request.0),
            Frame::McpListResources(request) => Ask::McpListResources(request.0),
            Frame::McpCallTool(request) => Ask::McpCallTool(request.0),
            Frame::McpReadResource(request) => Ask::McpReadResource(request.0),
            Frame::McpNotifications(_) => Ask::McpNotifications,
            Frame::FuseRead(target) => Ask::FuseRead(target.id.to_string(), target.path.to_string()),
            Frame::FuseWrite(request) => Ask::FuseWrite(
                request.id.to_string(),
                request.path.to_string(),
                Bytes::copy_from_slice(request.bytes),
            ),
            Frame::FuseList(target) => Ask::FuseList(target.id.to_string(), target.path.to_string()),
            Frame::FuseRemove(target) => {
                Ask::FuseRemove(target.id.to_string(), target.path.to_string())
            }
            Frame::FuseRename(request) => Ask::FuseRename(
                request.id.to_string(),
                request.from.to_string(),
                request.to.to_string(),
            ),
            Frame::FuseMkdir(target) => Ask::FuseMkdir(target.id.to_string(), target.path.to_string()),
            Frame::FuseStat(target) => Ask::FuseStat(target.id.to_string(), target.path.to_string()),
        }
    }
}

impl From<tools::run::server::channel_request::Frame<'_>> for Ask {
    fn from(frame: tools::run::server::channel_request::Frame<'_>) -> Self {
        use tools::run::server::channel_request::Frame;
        match frame {
            Frame::OciManifest(request) => Ask::OciManifest(request.digest),
            Frame::OciBlob(request) => Ask::OciBlob(request.digest),
            Frame::OciHas(request) => Ask::OciHas(request.name, request.digest),
            Frame::Authorize(request) => Ask::Authorize(request),
            Frame::Write(request) => Ask::Write(request.write_id),
            Frame::Postgres(request) => Ask::Postgres(request.connection_id),
            Frame::Command(request) => Ask::Command(Bytes::copy_from_slice(request.0)),
            Frame::VaultGet(request) => Ask::VaultGet(request.key.to_string()),
            Frame::VaultSet(request) => {
                Ask::VaultSet(request.key.to_string(), Bytes::copy_from_slice(request.value))
            }
            Frame::VaultDelete(request) => Ask::VaultDelete(request.key.to_string()),
            Frame::VaultLock(request) => Ask::VaultLock(request.key.to_string(), request.ttl),
            Frame::VaultUnlock(request) => Ask::VaultUnlock(request.key.to_string()),
            Frame::McpListTools(request) => Ask::McpListTools(request.0),
            Frame::McpListResources(request) => Ask::McpListResources(request.0),
            Frame::McpCallTool(request) => Ask::McpCallTool(request.0),
            Frame::McpReadResource(request) => Ask::McpReadResource(request.0),
            Frame::McpNotifications(_) => Ask::McpNotifications,
            Frame::FuseRead(target) => Ask::FuseRead(target.id.to_string(), target.path.to_string()),
            Frame::FuseWrite(request) => Ask::FuseWrite(
                request.id.to_string(),
                request.path.to_string(),
                Bytes::copy_from_slice(request.bytes),
            ),
            Frame::FuseList(target) => Ask::FuseList(target.id.to_string(), target.path.to_string()),
            Frame::FuseRemove(target) => {
                Ask::FuseRemove(target.id.to_string(), target.path.to_string())
            }
            Frame::FuseRename(request) => Ask::FuseRename(
                request.id.to_string(),
                request.from.to_string(),
                request.to.to_string(),
            ),
            Frame::FuseMkdir(target) => Ask::FuseMkdir(target.id.to_string(), target.path.to_string()),
            Frame::FuseStat(target) => Ask::FuseStat(target.id.to_string(), target.path.to_string()),
        }
    }
}
