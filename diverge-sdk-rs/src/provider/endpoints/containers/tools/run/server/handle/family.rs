//! This scope's frames, named for the machinery.

use std::borrow::Cow;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;

use bytes::Bytes;

use super::super::super::client::channel_request;
use super::super::super::client::channel_response::write_bytes;
use super::super::channel_response::{filetree, read, transfer, write_path};
use super::super::{channel_request as ask, response};
use crate::wire::client::handle::Handle;
use crate::container_proxy::outside::client::Ask;
use crate::container_proxy::outside::fuse::mount::client::execute::Ask as MountAsk;
use crate::wire::decode::Decode as _;
use crate::provider::endpoints::containers::server::begin::{Begin, Begun};
use crate::provider::endpoints::containers::server::family::{Family, Opened, Runs};
use crate::provider::endpoints::containers::server::own::Own;
use crate::provider::endpoints::containers::server::run::Run;
use crate::provider::endpoints::containers::server::serve::tool;
use crate::provider::endpoints::containers::server::{encoded::encoded, render};
use crate::container_proxy::outside::tools::begin::client::execute as begin;
use crate::shared;
use crate::shared::containers::request::Image;
use crate::shared::containers::response::{Id, VolumeHeld};
use crate::shared::containers::{command, fuse, oci, postgres, tools, vault};
use crate::shared::mcp;
use crate::shared::error::Error;
use crate::shared::filetree as tree;

/// A tool container run: the five MCP exchanges are the family's own.
pub(crate) struct Tools;

impl Family for Tools {
    type Request = channel_request::Frame;
    type Exchange = tool::Exchange;

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
            channel_request::Frame::Schema => Opened::Schema,
            channel_request::Frame::McpListTools(request) => Opened::Exchange(tool::Exchange::ListTools(request)),
            channel_request::Frame::McpListResources(request) => {
                Opened::Exchange(tool::Exchange::ListResources(request))
            }
            channel_request::Frame::McpCallTool(request) => Opened::Exchange(tool::Exchange::CallTool(request)),
            channel_request::Frame::McpReadResource(request) => Opened::Exchange(tool::Exchange::ReadResource(request)),
            channel_request::Frame::McpNotifications(_) => Opened::Exchange(tool::Exchange::Notifications),
        }
    }

    fn serve(run: Arc<Run>, channel: u32, exchange: Self::Exchange) -> impl Future<Output = ()> + Send + 'static {
        tool::serve(run, channel, exchange)
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

impl Runs for Tools {
    type Ask<'a> = ask::Frame<'a>;

    fn begin(proxy: &Handle, arguments: Value, image: Image) -> impl Future<Output = Result<Begun, Error>> + Send {
        let proxy = proxy.clone();
        async move {
            match begin::execute(&proxy, arguments, image).await {
                Ok((handle, asks, finish, tools)) => Ok(Begun {
                    begin: Begin::Tools(handle),
                    asks,
                    chunks: None,
                    finish: Some(finish),
                    tools,
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

    fn volume_held(refused: &VolumeHeld) -> Option<Vec<u8>> {
        encoded(&response::Frame::VolumeHeld(refused.clone()))
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
            Own::Authorize(authorize) => ask::Frame::Authorize(authorize),
            Own::Tools(declared) => ask::Frame::Tools(tools::request::Request {
                tools: Cow::Borrowed(declared),
            }),
            Own::Postgres(connection_id) => ask::Frame::Postgres(postgres::request::Postgres { connection_id }),
        }
    }
}
