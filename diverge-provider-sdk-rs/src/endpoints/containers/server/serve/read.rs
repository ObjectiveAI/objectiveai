//! One file out of the container, for the caller.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use crate::container_proxy::filesystem::read;

/// The file's pieces as the proxy sends them, then the finish. A file
/// that cannot be had is one `Error`, then the finish; a proxy that
/// could not serve the read at all is the finish alone.
pub(crate) async fn read<F: Family>(run: Arc<Run>, channel: u32, path: Vec<String>) {
    let request = read::request::Request { path };
    match read::execute::execute(&run.client, &request).await {
        Err(error) => run.respond(channel, F::read_error(&render::proxy(error))).await,
        Ok(mut pieces) => {
            while let Some(item) = pieces.next().await {
                match item {
                    Ok(bytes) => run.respond(channel, F::read_body(&bytes)).await,
                    Err(read::execute::ExecuteStreamError::Unserved) => break,
                    Err(read::execute::ExecuteStreamError::Refused(message)) => {
                        run.respond(channel, F::read_error(&render::refused(&message))).await;
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
