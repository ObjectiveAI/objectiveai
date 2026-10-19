//! A watch of a container's tree, whole.

use std::future::Future;
use std::pin::Pin;

use diverge_sdk::shared::filetree::response::{Frame, Root};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;

use super::cancelled;
use crate::containers::{Opened, files};

/// How a family sends one frame of the tree.
pub type SendFrame<'a> = &'a (dyn for<'b> Fn(&'b ScopeHandle, Frame) -> Pin<Box<dyn Future<Output = ()> + Send + 'b>> + Sync);

/// Send a snapshot of the container's whole tree — the provider's
/// with every mount of the daemon's spliced in — then every change
/// as it comes, the provider's frames and the mounts' own, until the
/// client cancels, the provider's stream ends — the run over — or it
/// fails, which is the error after what was sent. A snapshot the
/// provider sends again is spliced again and sent whole.
pub async fn watch(scope: &ScopeHandle, opened: &Opened, send: SendFrame<'_>) -> Result<(), String> {
    let mut provider = opened.filetree().await?;
    let mut changes = opened.mounts().subscribe();
    let mut root = Root(Vec::new());
    loop {
        tokio::select! {
            frame = provider.next() => match frame {
                Some(Ok(Frame::Snapshot { children })) => {
                    root = Root(children);
                    for (path, node) in opened.mounts().subtrees().await {
                        root.update(Frame::Inserted { path, node });
                    }
                    send(scope, Frame::Snapshot { children: root.0.clone() }).await;
                }
                Some(Ok(frame)) => {
                    root.update(frame.clone());
                    send(scope, frame).await;
                }
                Some(Err(error)) => return Err(error),
                None => return Ok(()),
            },
            change = changes.recv() => match change {
                Ok(frame) => {
                    root.update(frame.clone());
                    send(scope, frame).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    // Changes were missed: the tree is sent whole again,
                    // as a source that lost track of the tree does.
                    match files::tree(opened).await {
                        Ok(children) => {
                            root = Root(children);
                            send(scope, Frame::Snapshot { children: root.0.clone() }).await;
                        }
                        Err(error) => return Err(error),
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(()),
            },
            () = cancelled(scope) => return Ok(()),
        }
    }
}
