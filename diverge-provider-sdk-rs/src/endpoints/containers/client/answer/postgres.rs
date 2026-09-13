//! A database connection the container opened, carried to the
//! caller's database.

use std::sync::Arc;

use futures_util::StreamExt as _;
use tokio::sync::mpsc;

use super::super::{Encoders, encoded};
use super::send::{Stop, finish, respond_pieces};
use crate::client::PostgresDialer;
use crate::client::handle::Handle;
use crate::frame;
use crate::shared::containers::postgres;

/// The pair: the dialer is offered the connection; declined, the
/// provider's channel is finished with nothing. Accepted, this end
/// opens its own half — the family's `Postgres { connection_id }`
/// channel request — and forwards it, frame by frame, into the
/// dialer's receiver until it finishes, while everything the database
/// says goes out on the provider's channel until the dialer's stream
/// ends; then the finish, which is the database closing the
/// connection. The forwarder is then waited for, not aborted: the
/// provider closes the driver's socket on that finish, so this end's
/// half ends too, and every byte the container wrote before it did
/// reaches the dialer.
pub(crate) async fn postgres<P: PostgresDialer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    connection_id: u32,
    dialer: Arc<P>,
    encoders: Encoders,
) -> Result<(), Stop> {
    let (from_container, receiver) = mpsc::unbounded_channel();
    let Some(from_database) = dialer.dial(connection_id, receiver).await else {
        return finish(handle, scope, channel).await;
    };
    let mut from_database = std::pin::pin!(from_database);
    // This end's half. Without it the dialer never hears the
    // container, so a half that cannot open is a dial declined.
    let Some(half) = (encoders.postgres_half)(connection_id) else {
        return finish(handle, scope, channel).await;
    };
    let Ok(mut own) = handle.send_channel_request(scope, &half).await else {
        return finish(handle, scope, channel).await;
    };
    let forward = tokio::spawn(async move {
        while let Some(bytes) = own.response_receiver.recv().await {
            match frame::server::ServerFrame::decode(&bytes) {
                Ok(frame::server::ServerFrame::ChannelResponse { payload, .. }) => {
                    if from_container.send(bytes.slice_ref(payload)).is_err() {
                        break;
                    }
                }
                // The finish: the container's socket ended. Dropping
                // the sender is how the dialer hears it.
                Ok(frame::server::ServerFrame::ChannelResponseFinish { .. }) => break,
                _ => break,
            }
        }
    });
    let sent = async {
        while let Some(piece) = from_database.next().await {
            respond_pieces(handle, scope, channel, &piece, |body| {
                encoded(&postgres::response::Frame(body))
            })
            .await?;
        }
        finish(handle, scope, channel).await
    }
    .await;
    // Not aborted: whatever the container wrote before its socket
    // closed is still in this end's half, and the dialer is owed it.
    let _ = forward.await;
    sent
}
