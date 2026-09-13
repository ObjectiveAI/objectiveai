//! The container's tree, watched: a scope on the proxy, its frames
//! relayed.

use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, Either};
use futures_util::StreamExt as _;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use crate::container_proxy_endpoints::filesystem::tree::client::execute as tree;

/// Open a `filesystem::tree` scope leaving every mount out, and put
/// every frame on the caller's channel as it comes; the proxy's error
/// last, then the finish. A tree does not end by itself, so the run
/// ending stops it before the task is aborted.
pub(crate) async fn filetree<F: Family>(run: Arc<Run>, channel: u32) {
    match tree::execute(&run.proxy, run.ignore.clone()).await {
        Err(error) => run.respond(channel, F::filetree_error(&render::proxy(error))).await,
        Ok((mut frames, handle)) => {
            let mut ending = pin!(run.ending.notified());
            ending.as_mut().enable();
            loop {
                let next = pin!(frames.next());
                match future::select(next, ending.as_mut()).await {
                    Either::Left((Some(Ok(frame)), _)) => run.respond(channel, F::filetree(frame)).await,
                    Either::Left((Some(Err(tree::ExecuteStreamError::Refused(error))), _)) => {
                        run.respond(channel, F::filetree_error(&error)).await;
                        break;
                    }
                    Either::Left((Some(Err(error)), _)) => {
                        run.respond(channel, F::filetree_error(&render::proxy(error))).await;
                        break;
                    }
                    Either::Left((None, _)) => break,
                    Either::Right(_) => {
                        let _ = handle.stop().await;
                        break;
                    }
                }
            }
        }
    }
    run.finish(channel).await;
}
