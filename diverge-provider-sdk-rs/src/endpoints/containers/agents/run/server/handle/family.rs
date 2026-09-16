//! This scope's frames, named for the machinery.

use std::future::Future;
use std::sync::Arc;

use serde_json::Value;

use bytes::Bytes;

use super::super::super::client::channel_request;
use super::super::super::client::channel_response::write_bytes;
use super::super::channel_response::{filetree, read, transfer, write_path};
use super::super::{channel_request as ask, response};
use crate::client::handle::Handle;
use crate::container_proxy_endpoints::client::Ask;
use crate::container_proxy_endpoints::fuse::mount::client::execute::Ask as MountAsk;
use crate::decode::Decode as _;
use crate::endpoints::containers::server::begin::{Begin, Begun};
use crate::endpoints::containers::server::family::{Family, Opened, Runs};
use crate::endpoints::containers::server::own::Own;
use crate::endpoints::containers::server::run::Run;
use crate::endpoints::containers::server::serve::agent;
use crate::endpoints::containers::server::{encoded::encoded, render};
use crate::container_proxy_endpoints::agents::begin::client::execute as begin;
use crate::shared;
use crate::shared::containers::response::{Id, VolumeMounted};
use crate::shared::containers::{command, fuse, oci, postgres, vault};
use crate::shared::mcp;
use crate::shared::error::Error;
use crate::shared::filetree as tree;

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
            channel_request::Frame::Postgres(request) => Opened::Postgres(request.connection_id),
            channel_request::Frame::AgentSchema => Opened::Exchange(agent::Exchange::AgentSchema),
            channel_request::Frame::Enqueue(request) => Opened::Exchange(agent::Exchange::Enqueue(request.prompt)),
            channel_request::Frame::Dequeue => Opened::Exchange(agent::Exchange::Dequeue),
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

    fn begin(proxy: &Handle, agent: Option<Value>) -> impl Future<Output = Result<Begun, Error>> + Send {
        let proxy = proxy.clone();
        async move {
            // The agents handle always passes one; an absent agent is
            // `null`, which the image is free to refuse.
            let agent = agent.unwrap_or_default();
            match begin::execute(&proxy, agent).await {
                Ok((handle, asks, chunks)) => Ok(Begun {
                    begin: Begin::Agents(handle),
                    asks,
                    chunks: Some(chunks),
                    finish: None,
                }),
                Err(begin::ExecuteError::Refused(error)) => Err(error),
                Err(error) => Err(render::proxy(error)),
            }
        }
    }

    fn relayed<'a>(ask: &'a Ask) -> Option<ask::Frame<'a>> {
        Some(relayed(ask)?)
    }

    fn fuse<'a>(mount_id: &'a str, ask: &'a MountAsk) -> ask::Frame<'a> {
        fuse(mount_id, ask)
    }

    fn id(id: &Id) -> Option<Vec<u8>> {
        encoded(&response::Frame::Id(id.clone()))
    }

    fn volume_mounted(refused: &VolumeMounted) -> Option<Vec<u8>> {
        encoded(&response::Frame::VolumeMounted(refused.clone()))
    }
}

/// The proxy's ask as this family's frame; a database connection is
/// re-asked under this end's own id, see `Own::Postgres`.
fn relayed(ask: &Ask) -> Option<ask::Frame<'_>> {
    Some(match ask {
        Ask::Postgres(_) => return None,
        Ask::Command(bytes) => ask::Frame::Command(command::request::Request(bytes)),
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
        MountAsk::Read { path } => ask::Frame::FuseRead(fuse::Target { id, path }),
        MountAsk::Write { path, bytes } => ask::Frame::FuseWrite(fuse::write::request::Request { id, path, bytes }),
        MountAsk::List { path } => ask::Frame::FuseList(fuse::Target { id, path }),
        MountAsk::Remove { path } => ask::Frame::FuseRemove(fuse::Target { id, path }),
        MountAsk::Rename { from, to } => ask::Frame::FuseRename(fuse::rename::request::Request { id, from, to }),
        MountAsk::Mkdir { path } => ask::Frame::FuseMkdir(fuse::Target { id, path }),
        MountAsk::Stat { path } => ask::Frame::FuseStat(fuse::Target { id, path }),
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
            Own::Authorize(authorize) => ask::Frame::Authorize(authorize),
            Own::Postgres(connection_id) => ask::Frame::Postgres(postgres::request::Postgres { connection_id }),
        }
    }
}
