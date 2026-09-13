//! A FUSE mount scope: one mount, made on the server's request, and
//! the scope its asks ride for the proxy's life.

use std::sync::Arc;

use diverge_provider_sdk::container_proxy_endpoints::fuse::mount::client::request;
use diverge_provider_sdk::container_proxy_endpoints::fuse::mount::server::response;
use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::containers::fuse;

use crate::encode::encoded;
use crate::filesystem::mount::{self, Kind};
use crate::paths;
use crate::proxy::Proxy;

/// Make the mount — on a blocking thread, since it opens `/dev/fuse`
/// and makes the mount point — and answer once it is serving: `Ok`,
/// and the scope stays open for the proxy's life, every ask the mount
/// makes a channel on it; or `Error` with why not, then the finish.
/// The scope is the mount: the asks name no id.
pub async fn mount(proxy: Arc<Proxy>, scope: ScopeHandle, frame: request::Frame) {
    let scope = Arc::new(scope);
    match make(&proxy, Arc::clone(&scope), frame).await {
        Ok(()) => {
            if let Some(payload) = encoded(&response::Frame::Ok) {
                scope.send_response(&payload).await;
            }
        }
        Err(reason) => {
            if let Some(payload) = encoded(&response::Frame::Error(&reason)) {
                scope.send_response(&payload).await;
            }
            scope.send_response_finish().await;
        }
    }
}

/// Make the mount, or say why not: a path that names the root or has
/// a component that is not a name, a path already mounted, or a
/// mount the host or the kernel would not make.
async fn make(proxy: &Proxy, scope: Arc<ScopeHandle>, frame: request::Frame) -> Result<(), String> {
    let Some(path) = paths::absolute(&frame.path) else {
        return Err("the path names the root, or has a component that is not a name".to_string());
    };
    if proxy.mounts.holds(&path) {
        return Err(format!("{} is already a mount", path.display()));
    }
    let kind = match frame.kind {
        fuse::Kind::File => Kind::File,
        fuse::Kind::Directory => Kind::Directory,
    };
    let handle = tokio::runtime::Handle::current();
    let mounted = {
        let path = path.clone();
        tokio::task::spawn_blocking(move || mount::mount(scope, handle, &path, kind))
            .await
            .map_err(|error| format!("mount: {error}"))?
            .map_err(|error| format!("mount: {error}"))?
    };
    proxy.mounts.insert(path, mounted)
}
