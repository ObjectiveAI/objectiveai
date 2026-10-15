//! A daemon connection carried as a pair of channels, served
//! in-process.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use diverge_sdk::shared::containers::daemon as pair;
use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::encode::{Encode as _, Writer};
use diverge_sdk::wire::frame::server::ServerFrame;
use futures_util::{Stream, stream};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::watch;

use super::client;
use crate::daemon::Daemon;
use crate::judge::Who;

/// The daemon's half of a relayed connection: its server frames, one
/// per item, for as long as the session lives.
pub type PairFrames = Pin<Box<dyn Stream<Item = pair::server::Owned> + Send>>;

/// Serve the far side's client frames, arriving on `from`, as a
/// connection carried inside the process for `who` — a container's
/// program, or another daemon come through a provider — by the same
/// session and dispatch a socket gets, no credential passing; what
/// comes back is the session's frames as the daemon's half of the
/// pair. Every frame received is noted on `touched` when one is given,
/// since a container's every frame is a use of it. The far side's
/// channel closing ends the session, and the session ending ends the
/// stream.
pub fn serve_pair(daemon: Arc<Daemon>, who: Who, mut from: UnboundedReceiver<pair::client::Owned>, touched: Option<watch::Sender<Instant>>) -> PairFrames {
    let (to_session, incoming) = mpsc::unbounded_channel::<Bytes>();
    let (outgoing, from_session) = mpsc::unbounded_channel::<Bytes>();
    tokio::spawn(async move {
        while let Some(owned) = from.recv().await {
            let mut bytes = Vec::new();
            if owned.as_frame().encode(&mut Writer::new(&mut bytes)).is_err() {
                continue;
            }
            if let Some(touched) = &touched {
                touched.send_replace(Instant::now());
            }
            if to_session.send(Bytes::from(bytes)).is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        client(Connection::Local { incoming, outgoing }, who, daemon).await;
    });
    Box::pin(stream::unfold(from_session, |mut from_session| async move {
        loop {
            let bytes = from_session.recv().await?;
            let Ok(frame) = ServerFrame::decode(&bytes) else {
                continue;
            };
            let Ok(frame) = pair::server::Frame::try_from(frame) else {
                continue;
            };
            return Some((pair::server::Owned::from(frame), from_session));
        }
    }))
}
