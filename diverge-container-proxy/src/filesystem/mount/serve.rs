//! `/fuse/mount`, served: one mount, made on the server's request.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::container_proxy::fuse::mount::{request, response};
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::shared::containers::fuse;
use futures_util::SinkExt as _;

use super::{Kind, Mounts, mount};
use crate::agent;
use crate::requests::Requests;
use crate::ws;

/// `/fuse/mount`. Accepted as many times as the server opens it, one
/// mount each.
pub async fn fuse_mount(
    State(requests): State<Arc<Requests>>,
    State(mounts): State<Arc<Mounts>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, requests, mounts))
        .into_response()
}

/// The first binary message is the mount; a close before it, or a
/// request that will not decode, is the clean close with nothing
/// before it. Then the mount is made — on a blocking thread, since
/// it opens `/dev/fuse` and makes the mount point — and the one
/// answer goes out only once it is serving: `Ok`, or `Error` with why
/// not. Then the close.
async fn serve(mut socket: WebSocket, requests: Arc<Requests>, mounts: Arc<Mounts>) {
    let request = ws::binary(&mut socket)
        .await
        .and_then(|bytes| request::Request::decode(&bytes).ok());
    let Some(request) = request else {
        let _ = socket.close().await;
        return;
    };
    let outcome = make(requests, &mounts, request).await;
    let frame = match &outcome {
        Ok(()) => response::Frame::Ok,
        Err(reason) => response::Frame::Error(reason),
    };
    agent::finish(socket, agent::encoded(&frame)).await;
}

/// Make the mount, or say why not: a path that names the root or has
/// a component that is not a name, a path already mounted, or a
/// mount the host or the kernel would not make.
async fn make(requests: Arc<Requests>, mounts: &Mounts, request: request::Request) -> Result<(), String> {
    if request.path.is_empty()
        || request
            .path
            .iter()
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err("the path names the root, or has an empty, `.` or `..` component".to_string());
    }
    let mut path = PathBuf::from("/");
    path.extend(&request.path);
    if mounts.holds(&path) {
        return Err(format!("{} is already a mount", path.display()));
    }
    let kind = match request.kind {
        fuse::Kind::File => Kind::File,
        fuse::Kind::Directory => Kind::Directory,
    };
    let handle = tokio::runtime::Handle::current();
    let mounted = tokio::task::spawn_blocking(move || mount(requests, handle, &request, kind))
        .await
        .map_err(|error| format!("mount: {error}"))?
        .map_err(|error| format!("mount: {error}"))?;
    mounts.insert(path, mounted)
}
