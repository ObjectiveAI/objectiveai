//! The container's tree, watched: a scope on the proxy, its frames
//! relayed.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use crate::container_proxy_endpoints::filesystem::tree::client::execute as tree;

/// Open a `filesystem::tree` scope leaving every mount out, and put
/// every frame on the caller's channel as it comes; the proxy's error
/// last, then the finish. A tree does not end by itself, so the scope
/// is registered with the run, which stops it when the run shuts
/// down; the proxy's finish then ends the stream.
pub(crate) async fn filetree<F: Family>(run: Arc<Run>, channel: u32) {
    match tree::execute(&run.proxy, run.ignore.clone()).await {
        Err(error) => run.respond(channel, F::filetree_error(&render::proxy(error))).await,
        Ok((mut frames, handle)) => {
            run.trees.lock().await.insert(handle.scope(), handle.clone());
            while let Some(item) = frames.next().await {
                match item {
                    Ok(frame) => run.respond(channel, F::filetree(frame)).await,
                    Err(tree::ExecuteStreamError::Refused(error)) => {
                        run.respond(channel, F::filetree_error(&error)).await;
                        break;
                    }
                    Err(error) => {
                        run.respond(channel, F::filetree_error(&render::proxy(error))).await;
                        break;
                    }
                }
            }
            run.trees.lock().await.remove(&handle.scope());
        }
    }
    run.finish(channel).await;
}
