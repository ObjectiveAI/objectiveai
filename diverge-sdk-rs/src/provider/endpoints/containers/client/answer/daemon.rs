use std::sync::Arc;

use futures_util::StreamExt as _;

use super::send::{Stop, finish, respond};
use crate::provider::client::Daemon;
use crate::wire::client::handle::Handle;
use crate::shared::containers::daemon::{request, response};

pub(crate) async fn daemon<D: Daemon>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    frame: request::Owned,
    daemon: Arc<D>,
) -> Result<(), Stop> {
    let frames = daemon.frame(frame).await;
    let mut frames = std::pin::pin!(frames);
    while let Some(item) = frames.next().await {
        match item {
            Ok(served) => respond(handle, scope, channel, &response::Frame::Served(served.as_served())).await?,
            Err(error) => {
                respond(handle, scope, channel, &response::Frame::Error(error)).await?;
                break;
            }
        }
    }
    finish(handle, scope, channel).await
}
