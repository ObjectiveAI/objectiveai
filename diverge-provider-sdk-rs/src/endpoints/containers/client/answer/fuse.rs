//! The files and directories mounted live, from the caller's server.

use std::sync::Arc;

use bytes::Bytes;

use super::send::{Stop, finish, respond};
use crate::client::FuseServer;
use crate::client::handle::Handle;
use crate::shared::containers::fuse;

/// One frame — the bytes, that there are none, or the error — then
/// the finish.
pub(crate) async fn read<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, server: Arc<F>) -> Result<(), Stop> {
    let answer = server.read(&id, &path).await;
    let frame = match &answer {
        Ok(Some(bytes)) => fuse::read::response::Frame::Present(bytes),
        Ok(None) => fuse::read::response::Frame::Missing,
        Err(message) => fuse::read::response::Frame::Error(message),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// Ok, or the error, then the finish.
pub(crate) async fn write<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, bytes: Bytes, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.write(&id, &path, bytes).await).await
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

/// Ok, or the error, then the finish.
pub(crate) async fn remove<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.remove(&id, &path).await).await
}

/// Ok, or the error, then the finish.
pub(crate) async fn rename<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, from: String, to: String, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.rename(&id, &from, &to).await).await
}

/// Ok, or the error, then the finish.
pub(crate) async fn mkdir<F: FuseServer>(handle: &Handle, scope: u32, channel: u32, id: String, path: String, server: Arc<F>) -> Result<(), Stop> {
    ack(handle, scope, channel, server.mkdir(&id, &path).await).await
}

/// One frame — the kind and size, that there is nothing there, or the
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

/// The one-frame answer every mutation shares.
async fn ack(handle: &Handle, scope: u32, channel: u32, result: Result<(), String>) -> Result<(), Stop> {
    let frame = match &result {
        Ok(()) => fuse::ack::Frame::Ok,
        Err(message) => fuse::ack::Frame::Error(message),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}
