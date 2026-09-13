//! This scope's frames, named for the machinery.

use std::sync::Arc;

use bytes::Bytes;

use super::super::super::client::channel_request;
use super::super::super::client::channel_response::write_bytes;
use super::super::channel_response::{filetree, read, write_path};
use super::super::{channel_request as ask, response};
use crate::container_proxy::requests::request::Request;
use crate::decode::Decode as _;
use crate::endpoints::containers::server::family::{Family, Opened, Runs};
use crate::endpoints::containers::server::own::Own;
use crate::endpoints::containers::server::run::Run;
use crate::endpoints::containers::server::serve::agent;
use crate::endpoints::containers::server::{encoded::encoded, render};
use crate::shared;
use crate::shared::containers::response::{Id, VolumeMounted};
use crate::shared::containers::{fetch_directory, fetch_file, oci, postgres};
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
            channel_request::Frame::Postgres(request) => Opened::Postgres(request.connection_id),
            channel_request::Frame::AgentRun(request) => Opened::Exchange(agent::Exchange::AgentRun(request.prompt)),
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
}

impl Runs for Agents {
    type Ask<'a> = ask::Frame<'a>;

    fn relayed<'a>(request: Request<'a>) -> Option<ask::Frame<'a>> {
        Some(match request {
            Request::McpListTools(request) => ask::Frame::McpListTools(request),
            Request::McpListResources(request) => ask::Frame::McpListResources(request),
            Request::McpCallTool(request) => ask::Frame::McpCallTool(request),
            Request::McpReadResource(request) => ask::Frame::McpReadResource(request),
            Request::McpNotifications(request) => ask::Frame::McpNotifications(request),
            Request::VaultGet(request) => ask::Frame::VaultGet(request),
            Request::VaultSet(request) => ask::Frame::VaultSet(request),
            Request::VaultDelete(request) => ask::Frame::VaultDelete(request),
            Request::VaultLock(request) => ask::Frame::VaultLock(request),
            Request::VaultUnlock(request) => ask::Frame::VaultUnlock(request),
            Request::Command(request) => ask::Frame::Command(request),
            // Re-asked under this end's own id: see `Own::Postgres`.
            Request::Postgres(_) => return None,
            Request::FuseRead(request) => ask::Frame::FuseRead(request),
            Request::FuseWrite(request) => ask::Frame::FuseWrite(request),
            Request::FuseList(request) => ask::Frame::FuseList(request),
            Request::FuseRemove(request) => ask::Frame::FuseRemove(request),
            Request::FuseRename(request) => ask::Frame::FuseRename(request),
            Request::FuseMkdir(request) => ask::Frame::FuseMkdir(request),
            Request::FuseStat(request) => ask::Frame::FuseStat(request),
        })
    }

    fn id(id: &Id) -> Option<Vec<u8>> {
        encoded(&response::Frame::Id(id.clone()))
    }

    fn volume_mounted(refused: &VolumeMounted) -> Option<Vec<u8>> {
        encoded(&response::Frame::VolumeMounted(refused.clone()))
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
            Own::FetchFile(identity) => ask::Frame::FetchFile(fetch_file::request::Request {
                identity: identity.to_string(),
            }),
            Own::FetchDirectory(identity) => ask::Frame::FetchDirectory(fetch_directory::request::Request {
                identity: identity.to_string(),
            }),
            Own::Postgres(connection_id) => ask::Frame::Postgres(postgres::request::Postgres { connection_id }),
        }
    }
}
