//! Answering one server-opened ask, through the caller's answerers.
//!
//! One module per exchange, each a function that does the whole
//! answer — the frames, then the finish — through the handle, and
//! [`answer`] the dispatch over [`Ask`]. Nothing here reports a
//! failure: an answer that cannot be sent is an answer that ended, and
//! the provider reads the channel's end for what it is. Nothing
//! retries. An answer that stopped before its finish — the provider
//! abandoned it, or a frame would not encode — is still finished here,
//! once, so neither end keeps the channel.

mod daemon;
mod fuse;
mod mcp;
pub(crate) mod oci;
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
use crate::shared::containers::request::Image;

/// Answer `ask` on `channel` of `scope`, whose image — the run's — is
/// `image`: what a manifest or a blob, asked by digest alone, belongs
/// to.
pub(crate) async fn answer<O, T, P, D, M, F>(
    handle: Handle,
    scope: u32,
    channel: u32,
    ask: Ask,
    image: Arc<Image>,
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
    let answered = match ask {
        Ask::OciManifest(digest) => oci::manifest(&handle, scope, channel, image, digest, answerers.oci).await,
        Ask::OciBlob(digest) => oci::blob(&handle, scope, channel, image, digest, answerers.oci).await,
        Ask::OciHas(image) => oci::has(&handle, scope, channel, image, answerers.oci).await,
        Ask::OciProve(request) => oci::prove(&handle, scope, channel, image, request, answerers.oci).await,
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
    if answered.is_err() {
        let _ = send::finish(&handle, scope, channel).await;
    }
}

/// Answer `ask` on `channel` of `scope` from `store` alone: what a
/// scope whose provider asks only for an image answers with — an
/// [`images::push`](crate::provider::endpoints::images::push), whose
/// image is `image`. The OCI asks are answered as a run answers them;
/// any other ask is one this end cannot serve, and is finished with
/// nothing before it.
pub(crate) async fn answer_oci<O: OciStore + 'static>(
    handle: Handle,
    scope: u32,
    channel: u32,
    ask: Ask,
    image: Arc<Image>,
    store: Arc<O>,
) {
    let answered = match ask {
        Ask::OciManifest(digest) => oci::manifest(&handle, scope, channel, image, digest, store).await,
        Ask::OciBlob(digest) => oci::blob(&handle, scope, channel, image, digest, store).await,
        Ask::OciHas(image) => oci::has(&handle, scope, channel, image, store).await,
        Ask::OciProve(request) => oci::prove(&handle, scope, channel, image, request, store).await,
        _ => Err(send::Stop),
    };
    if answered.is_err() {
        let _ = send::finish(&handle, scope, channel).await;
    }
}
