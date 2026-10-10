//! The container's `/daemon` connections, served as its account or
//! under its template's grants.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::provider::client::Daemon as DaemonAnswerer;
use diverge_sdk::shared::containers::daemon as pair;
use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::encode::{Encode as _, Writer};
use diverge_sdk::wire::frame::server::ServerFrame;
use futures_util::{Stream, stream};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use super::Answerer;
use crate::judge::Who;
use crate::serve;

/// A connection the container's program opened, taken when the
/// container has a standing — a record's account, read fresh for
/// every request, or a dependency's grants, fixed at its deploy — and
/// declined when it has none: the program's frames go in as a
/// connection carried inside the process, served by the same session
/// and dispatch a socket gets, for that standing — no credential
/// passes — and the session's frames come back out as the daemon's
/// half of the pair. Every frame the program sends is a use of the
/// container.
impl DaemonAnswerer for Answerer {
    type Frames = Pin<Box<dyn Stream<Item = pair::server::Owned> + Send>>;

    async fn connect(&self, _: u32, mut from_program: UnboundedReceiver<pair::client::Owned>) -> Option<Self::Frames> {
        let who = match (&self.standing, self.account) {
            (Some(standing), _) => Who::Dependency(Arc::clone(standing)),
            (None, Some(account)) => Who::Account(account),
            (None, None) => return None,
        };
        let (to_session, incoming) = mpsc::unbounded_channel::<Bytes>();
        let (outgoing, from_session) = mpsc::unbounded_channel::<Bytes>();
        let touched = self.touched.clone();
        tokio::spawn(async move {
            while let Some(owned) = from_program.recv().await {
                let mut bytes = Vec::new();
                if owned.as_frame().encode(&mut Writer::new(&mut bytes)).is_err() {
                    continue;
                }
                touched.send_replace(std::time::Instant::now());
                if to_session.send(Bytes::from(bytes)).is_err() {
                    break;
                }
            }
        });
        let daemon = Arc::clone(&self.daemon);
        tokio::spawn(async move {
            serve::client(Connection::Local { incoming, outgoing }, who, daemon).await;
        });
        Some(Box::pin(stream::unfold(from_session, |mut from_session| async move {
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
        })))
    }
}
