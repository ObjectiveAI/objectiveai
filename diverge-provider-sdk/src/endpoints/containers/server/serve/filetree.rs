//! The container's filesystem, watched for the caller.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use crate::container_proxy::filesystem::tree;

/// A snapshot, then a frame per change, for as long as the proxy
/// sends them; then the finish. The tree that cannot be had is one
/// `Error`, then the finish.
pub(crate) async fn filetree<F: Family>(run: Arc<Run>, channel: u32) {
    match tree::execute::execute(&run.client).await {
        Err(error) => run.respond(channel, F::filetree_error(&render::proxy(error))).await,
        Ok(mut frames) => {
            while let Some(item) = frames.next().await {
                match item {
                    Ok(frame) => run.respond(channel, F::filetree(frame)).await,
                    Err(tree::execute::ExecuteStreamError::Refused(message)) => {
                        run.respond(channel, F::filetree_error(&render::refused(&message))).await;
                        break;
                    }
                    Err(error) => {
                        run.respond(channel, F::filetree_error(&render::proxy(error))).await;
                        break;
                    }
                }
            }
        }
    }
    run.finish(channel).await;
}
