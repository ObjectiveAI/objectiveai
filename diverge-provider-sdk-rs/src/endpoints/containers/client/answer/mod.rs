//! Answering one server-opened ask, through the caller's answerers.
//!
//! One module per exchange, each a function that does the whole
//! answer — the frames, then the finish — through the handle, and
//! [`answer`] the dispatch over [`Ask`]. Nothing here reports a
//! failure: an answer that cannot be sent is an answer that ended, and
//! the provider reads the channel's end for what it is. Nothing
//! retries.

mod authorize;
mod command;
mod fuse;
mod mcp;
mod oci;
mod postgres;
mod send;
mod tools;
mod vault;
mod write;

use std::sync::Arc;

/// The write answer alone, for the connect scope: the one ask a
/// provider makes of a connector.
pub(crate) use write::write as write_content;

use super::{Ask, Encoders, Writes};
use crate::client::handle::Handle;
use crate::client::{
    Answerers, CommandRunner, ConnectionAuthorizer, FuseServer, McpServer, OciStore, PostgresDialer, ToolDeployer,
    Vault,
};

/// Answer `ask` on `channel` of `scope`.
pub(crate) async fn answer<O, A, T, P, C, V, M, F>(
    handle: Handle,
    scope: u32,
    channel: u32,
    ask: Ask,
    writes: Arc<Writes>,
    answerers: Answerers<O, A, T, P, C, V, M, F>,
    encoders: Encoders,
) where
    O: OciStore + 'static,
    A: ConnectionAuthorizer + 'static,
    T: ToolDeployer + 'static,
    P: PostgresDialer + 'static,
    C: CommandRunner + 'static,
    V: Vault + 'static,
    M: McpServer + 'static,
    F: FuseServer + 'static,
{
    let _ = match ask {
        Ask::OciManifest(digest) => oci::manifest(&handle, scope, channel, digest, answerers.oci).await,
        Ask::OciBlob(digest) => oci::blob(&handle, scope, channel, digest, answerers.oci).await,
        Ask::OciHas(name, digest) => oci::has(&handle, scope, channel, name, digest, answerers.oci).await,
        Ask::Authorize(request) => {
            authorize::authorize(&handle, scope, channel, request, answerers.authorizer).await
        }
        Ask::Tools(declared) => tools::tools(&handle, scope, channel, declared, answerers.tools).await,
        Ask::Write(write_id) => write::write(&handle, scope, channel, write_id, writes, encoders).await,
        Ask::Postgres(connection_id) => {
            postgres::postgres(&handle, scope, channel, connection_id, answerers.postgres, encoders).await
        }
        Ask::Command(command) => command::command(&handle, scope, channel, command, answerers.commands).await,
        Ask::VaultGet(key) => vault::get(&handle, scope, channel, key, answerers.vault).await,
        Ask::VaultSet(key, value) => vault::set(&handle, scope, channel, key, value, answerers.vault).await,
        Ask::VaultDelete(key) => vault::delete(&handle, scope, channel, key, answerers.vault).await,
        Ask::VaultLock(key, ttl) => vault::lock(&handle, scope, channel, key, ttl, answerers.vault).await,
        Ask::VaultUnlock(key) => vault::unlock(&handle, scope, channel, key, answerers.vault).await,
        Ask::McpListTools(params) => mcp::list_tools(&handle, scope, channel, params, answerers.mcp).await,
        Ask::McpListResources(params) => {
            mcp::list_resources(&handle, scope, channel, params, answerers.mcp).await
        }
        Ask::McpCallTool(params) => mcp::call_tool(&handle, scope, channel, params, answerers.mcp).await,
        Ask::McpReadResource(params) => {
            mcp::read_resource(&handle, scope, channel, params, answerers.mcp).await
        }
        Ask::McpNotifications => mcp::notifications(&handle, scope, channel, answerers.mcp).await,
        Ask::FuseRead(id, path) => fuse::read(&handle, scope, channel, id, path, answerers.fuse).await,
        Ask::FuseWrite(id, path, bytes) => {
            fuse::write(&handle, scope, channel, id, path, bytes, answerers.fuse).await
        }
        Ask::FuseList(id, path) => fuse::list(&handle, scope, channel, id, path, answerers.fuse).await,
        Ask::FuseRemove(id, path) => fuse::remove(&handle, scope, channel, id, path, answerers.fuse).await,
        Ask::FuseRename(id, from, to) => {
            fuse::rename(&handle, scope, channel, id, from, to, answerers.fuse).await
        }
        Ask::FuseMkdir(id, path) => fuse::mkdir(&handle, scope, channel, id, path, answerers.fuse).await,
        Ask::FuseStat(id, path) => fuse::stat(&handle, scope, channel, id, path, answerers.fuse).await,
    };
}
