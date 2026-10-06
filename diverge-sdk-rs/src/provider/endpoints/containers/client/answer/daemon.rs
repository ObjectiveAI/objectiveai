use std::sync::Arc;

use futures_util::StreamExt as _;
use tokio::sync::mpsc;

use super::super::{Encoders, encoded};
use super::send::{Stop, finish, respond_bytes};
use crate::provider::client::Daemon;
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;
use crate::wire::frame;
use crate::shared::containers::daemon;

pub(crate) async fn daemon<D: Daemon>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    connection_id: u32,
    session: Arc<D>,
    encoders: Encoders,
) -> Result<(), Stop> {
    // The half this end opens is built first: a family that cannot
    // open one — a connector, whose container's asks go to its runner
    // — refuses the connection without a session having been made for
    // it.
    let Some(half) = (encoders.daemon_half)(connection_id) else {
        return finish(handle, scope, channel).await;
    };
    let (from_program, receiver) = mpsc::unbounded_channel();
    let Some(from_daemon) = session.connect(connection_id, receiver).await else {
        return finish(handle, scope, channel).await;
    };
    let mut from_daemon = std::pin::pin!(from_daemon);
    let Ok(mut own) = handle.send_channel_request(scope, &half).await else {
        return finish(handle, scope, channel).await;
    };
    let forward = tokio::spawn(async move {
        while let Some(bytes) = own.response_receiver.recv().await {
            match frame::server::ServerFrame::decode(&bytes) {
                Ok(frame::server::ServerFrame::ChannelResponse { payload, .. }) => {
                    let Ok(frame) = daemon::client::Frame::decode(payload) else {
                        break;
                    };
                    if from_program.send(daemon::client::Owned::from(frame)).is_err() {
                        break;
                    }
                }
                Ok(frame::server::ServerFrame::ChannelResponseFinish { .. }) => break,
                _ => break,
            }
        }
    });
    let sent = async {
        while let Some(frame) = from_daemon.next().await {
            let Some(bytes) = encoded(&frame.as_frame()) else {
                break;
            };
            respond_bytes(handle, scope, channel, &bytes).await?;
        }
        finish(handle, scope, channel).await
    }
    .await;
    let _ = forward.await;
    sent
}
