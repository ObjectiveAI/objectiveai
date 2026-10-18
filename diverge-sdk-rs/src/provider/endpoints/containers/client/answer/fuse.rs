//! The files and directories mounted live, from the caller's server.

use std::sync::Arc;

use bytes::Bytes;

use super::send::{Stop, finish, respond};
use crate::provider::client::FuseServer;
use crate::wire::client::handle::Handle;
use crate::shared::containers::fuse;
use crate::shared::containers::fuse::ack::Refused;

/// One frame — the piece, that there is no file, or the error — then
/// the finish.
pub(crate) async fn read<F: FuseServer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    id: String,
    path: String,
    offset: u64,
    length: u32,
    server: Arc<F>,
) -> Result<(), Stop> {
    let answer = server.read(&id, &path, offset, length).await;
    let frame = match &answer {
        Ok(Some(bytes)) => fuse::read::response::Frame::Present(bytes),
        Ok(None) => fuse::read::response::Frame::Missing,
        Err(message) => fuse::read::response::Frame::Error(message),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// Ok, or why not, then the finish.
pub(crate) async fn write<F: FuseServer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    id: String,
    path: String,
    offset: u64,
    bytes: Bytes,
    server: Arc<F>,
) -> Result<(), Stop> {
    ack(handle, scope, channel, server.write(&id, &path, offset, bytes).await).await
}

/// Ok, or why not, then the finish.
pub(crate) async fn truncate<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, size: u64, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.truncate(&id, &path, size).await).await
}

/// Ok, or why not, then the finish.
pub(crate) async fn setattr<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, attrs: fuse::Attrs, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.setattr(&id, &path, attrs).await).await
}

/// One frame — the entries, that there is no such directory, or the
/// error — then the finish.
pub(crate) async fn list<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, server: Arc<F>) -> Result<(), Stop> {
    let answer = server.list(&id, &path).await;
    let frame = match &answer {
        Ok(Some(listed)) => fuse::list::response::Frame::Entries(
            listed
                .iter()
                .map(|entry| fuse::Entry {
                    name: &entry.name,
                    kind: entry.kind,
                })
                .collect(),
        ),
        Ok(None) => fuse::list::response::Frame::Missing,
        Err(message) => fuse::list::response::Frame::Error(message),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// Ok, or why not, then the finish.
pub(crate) async fn remove<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.remove(&id, &path).await).await
}

/// Ok, or why not, then the finish.
pub(crate) async fn rename<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, from: String, to: String, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.rename(&id, &from, &to).await).await
}

/// Ok, or why not, then the finish.
pub(crate) async fn mkdir<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.mkdir(&id, &path).await).await
}

/// One frame — the attributes, that there is nothing there, or the
/// error — then the finish.
pub(crate) async fn stat<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, server: Arc<F>) -> Result<(), Stop> {
    let answer = server.stat(&id, &path).await;
    let frame = match &answer {
        Ok(Some(stat)) => fuse::stat::response::Frame::Present(*stat),
        Ok(None) => fuse::stat::response::Frame::Missing,
        Err(message) => fuse::stat::response::Frame::Error(message),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// The one-frame answer every mutation shares: ok, read only, or the
/// error.
async fn ack(handle: &Handle, scope: u32, channel: u32, result: Result<(), Refused>) -> Result<(), Stop> {
    let frame = match &result {
        Ok(()) => fuse::ack::Frame::Ok,
        Err(Refused::ReadOnly) => fuse::ack::Frame::ReadOnly,
        Err(Refused::Error(message)) => fuse::ack::Frame::Error(message),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}
