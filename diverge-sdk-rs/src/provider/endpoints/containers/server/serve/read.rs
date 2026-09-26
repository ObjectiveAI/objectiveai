//! One file, read out of the container: a scope on the proxy, its
//! pieces relayed.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use crate::container_proxy::outside::filesystem::read::client::execute as read;

/// Open a `filesystem::read` scope naming the file, and put every
/// piece on the caller's channel as it comes; the proxy's error last,
/// then the finish. A proxy that finishes with nothing is relayed as
/// nothing: the finish alone, the wire's could-not-serve.
pub(crate) async fn read<F: Family>(run: Arc<Run>, channel: u32, path: Vec<String>) {
    match read::execute(&run.proxy, path).await {
        Err(error) => run.respond(channel, F::read_error(&render::proxy(error))).await,
        Ok(mut pieces) => {
            while let Some(item) = pieces.next().await {
                match item {
                    Ok(bytes) => run.respond(channel, F::read_body(&bytes)).await,
                    Err(read::ExecuteStreamError::Refused(error)) => {
                        run.respond(channel, F::read_error(&error)).await;
                        break;
                    }
                    Err(error) => {
                        run.respond(channel, F::read_error(&render::proxy(error))).await;
                        break;
                    }
                }
            }
        }
    }
    run.finish(channel).await;
}
