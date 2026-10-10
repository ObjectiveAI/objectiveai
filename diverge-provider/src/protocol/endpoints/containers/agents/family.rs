//! This scope's frames, named for the machinery.

use std::borrow::Cow;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;

use bytes::Bytes;

use diverge_sdk::provider::endpoints::containers::agents::run::client::channel_request;
use diverge_sdk::provider::endpoints::containers::agents::run::client::channel_response::write_bytes;
use diverge_sdk::provider::endpoints::containers::agents::run::server::channel_response::{filetree, read, transfer, write_path};
use diverge_sdk::provider::endpoints::containers::agents::run::server::{channel_request as ask, response};
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::container_proxy::outside::client::Ask;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::client::execute::Ask as MountAsk;
use diverge_sdk::wire::decode::Decode as _;
use crate::protocol::endpoints::containers::run::begin::{Begin, Begun};
use crate::protocol::endpoints::containers::run::family::{Family, Opened, Runs};
use crate::protocol::endpoints::containers::run::own::Own;
use crate::protocol::endpoints::containers::run::run::Run;
use crate::protocol::endpoints::containers::run::serve::agent;
use crate::protocol::endpoints::containers::run::setup::deployed;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::endpoints::containers::run::{encoded::encoded, render};
use diverge_sdk::container_proxy::outside::endpoints::agents::begin::client::execute as begin;
use diverge_sdk::shared;
use diverge_sdk::shared::containers::response::{Id, VolumeHeld, VolumeMode};
use diverge_sdk::shared::containers::{dependencies, fuse, oci, postgres, vault};
use diverge_sdk::shared::containers::daemon;
use diverge_sdk::shared::mcp;
use diverge_sdk::shared::error::Error;
use diverge_sdk::shared::filetree as tree;

/// An agent container run: the loop and its queue are the family's own.
pub(crate) struct Agents;

impl Family for Agents {
    type Request = channel_request::Frame;
    type Exchange = agent::Exchange;

    fn classify(request: channel_request::Frame) -> Opened<Self::Exchange> {
        match request {
            channel_request::Frame::Stop => Opened::Stop,
            channel_request::Frame::Filetree => Opened::Filetree,
            channel_request::Frame::Read(request) => Opened::Read(request.path),
            channel_request::Frame::Write(request) => Opened::Write {
                write_id: request.write_id,
                path: request.path,
            },
            channel_request::Frame::Transfer(request) => Opened::Transfer {
                path: request.path,
                id: request.id,
                destination: request.destination,
            },
            channel_request::Frame::Daemon(request) => Opened::Daemon(request.connection_id),
            channel_request::Frame::Postgres(request) => Opened::Postgres(request.connection_id),
            channel_request::Frame::Schema => Opened::Schema,
            channel_request::Frame::Enqueue(request) => Opened::Exchange(agent::Exchange::Enqueue(request.key, request.content)),
            channel_request::Frame::Dequeue(request) => Opened::Exchange(agent::Exchange::Dequeue(request.key)),
        }
    }

    fn serve(run: Arc<Run>, channel: u32, exchange: Self::Exchange) -> impl Future<Output = ()> + Send + 'static {
        agent::serve(run, channel, exchange)
    }

    fn error(error: &Error) -> Option<Vec<u8>> {
        encoded(&response::Frame::Error(error.clone()))
    }

    fn write_ask(write_id: u32) -> Option<Vec<u8>> {
        encoded(&ask::Frame::Write(shared::containers::write_bytes::request::Request { write_id }))
    }

    fn content(payload: &Bytes) -> Result<Bytes, Error> {
        match write_bytes::Frame::decode(payload) {
            Ok(write_bytes::Frame::Body(body)) => Ok(payload.slice_ref(body.0)),
            Ok(write_bytes::Frame::Error(error)) => Err(error),
            Err(error) => Err(render::proxy(error)),
        }
    }

    fn filetree(frame: tree::response::Frame) -> Option<Vec<u8>> {
        encoded(&filetree::Frame::Filetree(frame))
    }

    fn filetree_error(error: &Error) -> Option<Vec<u8>> {
        encoded(&filetree::Frame::Error(error.clone()))
    }

    fn read_body(bytes: &[u8]) -> Option<Vec<u8>> {
        encoded(&read::Frame::Body(shared::containers::read::response::Frame(bytes)))
    }

    fn read_error(error: &Error) -> Option<Vec<u8>> {
        encoded(&read::Frame::Error(error.clone()))
    }

    fn written() -> Option<Vec<u8>> {
        encoded(&write_path::Frame::Written(shared::containers::write_path::response::Frame))
    }

    fn write_error(error: &Error) -> Option<Vec<u8>> {
        encoded(&write_path::Frame::Error(error.clone()))
    }

    fn transferred() -> Option<Vec<u8>> {
        encoded(&transfer::Frame::Transferred(shared::containers::transfer::response::Frame))
    }

    fn transfer_error(error: &Error) -> Option<Vec<u8>> {
        encoded(&transfer::Frame::Error(error.clone()))
    }
}

impl Runs for Agents {
    type Ask<'a> = ask::Frame<'a>;

    fn begin(proxy: &Handle, arguments: Value) -> impl Future<Output = Result<Begun, Error>> + Send {
        let proxy = proxy.clone();
        async move {
            match begin::execute(&proxy, arguments).await {
                Ok((handle, asks, chunks, dependencies)) => Ok(Begun {
                    begin: Begin::Agents(handle),
                    asks,
                    chunks: Some(chunks),
                    finish: None,
                    dependencies,
                }),
                Err(begin::ExecuteError::Refused(error)) => Err(error),
                Err(error) => Err(render::proxy(error)),
            }
        }
    }

    fn relayed<'a>(ask: &'a Ask) -> Option<ask::Frame<'a>> {
        Some(relayed(ask)?)
    }

    /// The dependencies the agent declared, asked of the caller on
    /// this family's `dependencies` channel: nothing when it declared
    /// none.
    fn deploy(scope: &ScopeHandle, id: &str, declared: &[dependencies::Template]) -> impl Future<Output = Result<(), Error>> + Send {
        let payload = (!declared.is_empty()).then(|| {
            encoded(&ask::Frame::Dependencies(dependencies::request::Request {
                id: Cow::Borrowed(id),
                dependencies: Cow::Borrowed(declared),
            }))
        });
        async move {
            match payload {
                None => Ok(()),
                Some(Some(payload)) => deployed(scope, &payload).await,
                Some(None) => Err(render::dependencies_failed("the dependencies ask did not encode")),
            }
        }
    }

    fn fuse<'a>(mount_id: &'a str, ask: &'a MountAsk) -> ask::Frame<'a> {
        fuse(mount_id, ask)
    }

    fn id(id: &Id) -> Option<Vec<u8>> {
        encoded(&response::Frame::Id(id.clone()))
    }

    fn volume_held(refused: &VolumeHeld) -> Option<Vec<u8>> {
        encoded(&response::Frame::VolumeHeld(refused.clone()))
    }

    fn volume_mode(refused: &VolumeMode) -> Option<Vec<u8>> {
        encoded(&response::Frame::VolumeMode(refused.clone()))
    }
}

/// The proxy's ask as this family's frame; a database connection is
/// re-asked under this end's own id, see `Own::Postgres`.
fn relayed(ask: &Ask) -> Option<ask::Frame<'_>> {
    Some(match ask {
        Ask::Postgres(_) => return None,
        Ask::Daemon(_) => return None,
        Ask::VaultGet { key } => ask::Frame::VaultGet(vault::get::request::Request { key }),
        Ask::VaultSet { key, value } => ask::Frame::VaultSet(vault::set::request::Request { key, value }),
        Ask::VaultDelete { key } => ask::Frame::VaultDelete(vault::delete::request::Request { key }),
        Ask::VaultLock { key, ttl } => ask::Frame::VaultLock(vault::lock::request::Request { key, ttl: *ttl }),
        Ask::VaultUnlock { key } => ask::Frame::VaultUnlock(vault::unlock::request::Request { key }),
        Ask::McpListTools(request) => ask::Frame::McpListTools(request.clone()),
        Ask::McpListResources(request) => ask::Frame::McpListResources(request.clone()),
        Ask::McpCallTool(request) => ask::Frame::McpCallTool(request.clone()),
        Ask::McpReadResource(request) => ask::Frame::McpReadResource(request.clone()),
        Ask::McpNotifications => ask::Frame::McpNotifications(mcp::notifications::request::Request),
    })
}

/// A mount's ask as this family's frame, the caller's id in front.
fn fuse<'a>(id: &'a str, ask: &'a MountAsk) -> ask::Frame<'a> {
    match ask {
        MountAsk::Read { path, offset, length } => ask::Frame::FuseRead(fuse::read::request::Request {
            id,
            path,
            offset: *offset,
            length: *length,
        }),
        MountAsk::Write { path, offset, bytes } => ask::Frame::FuseWrite(fuse::write::request::Request {
            id,
            path,
            offset: *offset,
            bytes,
        }),
        MountAsk::List { path } => ask::Frame::FuseList(fuse::Target { id, path }),
        MountAsk::Remove { path } => ask::Frame::FuseRemove(fuse::Target { id, path }),
        MountAsk::Rename { from, to } => ask::Frame::FuseRename(fuse::rename::request::Request { id, from, to }),
        MountAsk::Mkdir { path } => ask::Frame::FuseMkdir(fuse::Target { id, path }),
        MountAsk::Stat { path } => ask::Frame::FuseStat(fuse::Target { id, path }),
        MountAsk::Truncate { path, size } => ask::Frame::FuseTruncate(fuse::truncate::request::Request { id, path, size: *size }),
        MountAsk::Setattr { path, attrs } => ask::Frame::FuseSetattr(fuse::setattr::request::Request { id, path, attrs: *attrs }),
    }
}

impl<'a> From<Own<'a>> for ask::Frame<'a> {
    fn from(own: Own<'a>) -> Self {
        match own {
            Own::OciManifest(digest) => ask::Frame::OciManifest(oci::manifest::request::Request {
                digest: digest.to_string(),
            }),
            Own::OciBlob(digest) => ask::Frame::OciBlob(oci::blob::request::Request {
                digest: digest.to_string(),
            }),
            Own::OciHas { name, digest } => ask::Frame::OciHas(oci::has::request::Request {
                name: name.to_string(),
                digest: digest.to_string(),
            }),
            Own::Daemon(connection_id) => ask::Frame::Daemon(daemon::request::Daemon { connection_id }),
            Own::Postgres(connection_id) => ask::Frame::Postgres(postgres::request::Postgres { connection_id }),
        }
    }
}
