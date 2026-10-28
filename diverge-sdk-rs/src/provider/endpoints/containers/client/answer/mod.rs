//! Answering one server-opened ask, through the caller's answerers.
//!
//! One module per exchange, each a function that does the whole
//! answer — the frames, then the finish — through the handle, and
//! [`answer`] the dispatch over [`Ask`]. Nothing here reports a
//! failure: an answer that cannot be sent is an answer that ended, and
//! the provider reads the channel's end for what it is. Nothing
//! retries.

mod daemon;
mod fuse;
mod mcp;
mod oci;
mod postgres;
mod registration;
mod send;
mod write;

use std::sync::Arc;

use super::{Ask, Encoders, Writes};
use crate::wire::client::handle::Handle;
use crate::provider::client::{
    Answerers, Daemon, FuseServer, McpServer, OciStore, PostgresDialer, Registrar,
};

/// Answer `ask` on `channel` of `scope`.
pub(crate) async fn answer<O, T, P, D, M, F>(
    handle: Handle,
    scope: u32,
    channel: u32,
    ask: Ask,
    writes: Arc<Writes>,
    answerers: Answerers<O, T, P, D, M, F>,
    encoders: Encoders,
) where
    O: OciStore + 'static,
    T: Registrar + 'static,
    P: PostgresDialer + 'static,
    D: Daemon + 'static,
    M: McpServer + 'static,
    F: FuseServer + 'static,
{
    let _ = match ask {
        Ask::OciManifest(digest) => oci::manifest(&handle, scope, channel, digest, answerers.oci).await,
        Ask::OciBlob(digest) => oci::blob(&handle, scope, channel, digest, answerers.oci).await,
        Ask::OciHas(digest) => oci::has(&handle, scope, channel, digest, answerers.oci).await,
        Ask::Registration { id, dependencies, secrets, permissions } => {
            registration::registration(&handle, scope, channel, id, dependencies, secrets, permissions, answerers.registrar).await
        }
        Ask::Write(write_id) => write::write(&handle, scope, channel, write_id, writes, encoders).await,
        Ask::Postgres(connection_id) => {
            postgres::postgres(&handle, scope, channel, connection_id, answerers.postgres, encoders).await
        }
        Ask::Daemon(connection_id) => {
            daemon::daemon(&handle, scope, channel, connection_id, answerers.daemon, encoders).await
        }
        Ask::McpListTools(params) => mcp::list_tools(&handle, scope, channel, params, answerers.mcp).await,
        Ask::McpListResources(params) => {
            mcp::list_resources(&handle, scope, channel, params, answerers.mcp).await
        }
        Ask::McpCallTool(params) => mcp::call_tool(&handle, scope, channel, params, answerers.mcp).await,
        Ask::McpReadResource(params) => {
            mcp::read_resource(&handle, scope, channel, params, answerers.mcp).await
        }
        Ask::McpNotifications => mcp::notifications(&handle, scope, channel, answerers.mcp).await,
        Ask::FuseRead(id, path, offset, length) => {
            fuse::read(&handle, scope, channel, id, path, offset, length, answerers.fuse).await
        }
        Ask::FuseWrite(id, path, offset, bytes) => {
            fuse::write(&handle, scope, channel, id, path, offset, bytes, answerers.fuse).await
        }
        Ask::FuseList(id, path) => fuse::list(&handle, scope, channel, id, path, answerers.fuse).await,
        Ask::FuseRemove(id, path) => fuse::remove(&handle, scope, channel, id, path, answerers.fuse).await,
        Ask::FuseRename(id, from, to) => {
            fuse::rename(&handle, scope, channel, id, from, to, answerers.fuse).await
        }
        Ask::FuseMkdir(id, path) => fuse::mkdir(&handle, scope, channel, id, path, answerers.fuse).await,
        Ask::FuseStat(id, path) => fuse::stat(&handle, scope, channel, id, path, answerers.fuse).await,
        Ask::FuseTruncate(id, path, size) => {
            fuse::truncate(&handle, scope, channel, id, path, size, answerers.fuse).await
        }
        Ask::FuseSetattr(id, path, attrs) => {
            fuse::setattr(&handle, scope, channel, id, path, attrs, answerers.fuse).await
        }
    };
}
