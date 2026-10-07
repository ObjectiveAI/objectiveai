//! The container's database connections: one scope of its own,
//! reached through the daemon.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::provider::client::PostgresDialer;
use futures_util::{Stream, stream};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use super::Answerer;
use crate::database::connect;

/// Every connection taken, and served on a task of its own as
/// [`connect`] states: the dial is answered at once with the stream
/// the task feeds, since the provider opens its half only after; the
/// handshake toward the container and toward the database happens on
/// the task, and a refusal is that stream ending with the error the
/// container's driver reads.
impl PostgresDialer for Answerer {
    type Connection = Pin<Box<dyn Stream<Item = Bytes> + Send>>;

    async fn dial(&self, _: u32, from_container: UnboundedReceiver<Bytes>) -> Option<Self::Connection> {
        let (to_container, receiver) = mpsc::unbounded_channel::<Bytes>();
        tokio::spawn(connect::run(
            Arc::clone(&self.daemon),
            self.key,
            from_container,
            to_container,
            self.touched.clone(),
        ));
        Some(Box::pin(stream::unfold(receiver, |mut receiver| async move {
            let bytes = receiver.recv().await?;
            Some((bytes, receiver))
        })))
    }
}
