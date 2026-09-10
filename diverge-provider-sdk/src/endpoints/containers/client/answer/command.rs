//! A command the container asked run, from the caller's runner.

use std::sync::Arc;

use bytes::Bytes;
use futures_util::StreamExt as _;

use super::send::{Stop, finish, respond};
use crate::client::CommandRunner;
use crate::client::handle::Handle;
use crate::shared::containers::command;

/// One frame per item, then the finish; an `Error` frame, last, for
/// a command that failed part-way.
pub(crate) async fn command<C: CommandRunner>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    command: Bytes,
    runner: Arc<C>,
) -> Result<(), Stop> {
    let items = runner.run(command).await;
    let mut items = std::pin::pin!(items);
    while let Some(item) = items.next().await {
        match item {
            Ok(item) => respond(handle, scope, channel, &command::response::Frame::Item(&item)).await?,
            Err(error) => {
                respond(handle, scope, channel, &command::response::Frame::Error(error)).await?;
                break;
            }
        }
    }
    finish(handle, scope, channel).await
}
