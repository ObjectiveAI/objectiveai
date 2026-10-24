//! The agent's conversation, from the begin's main stream onto the
//! run's.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::{Run, send};
use diverge_sdk::container_proxy::outside::endpoints::agents::begin::client::execute::{Chunks, ChunksError};
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::Event;
use diverge_sdk::provider::endpoints::containers::agents::run::server::response;

/// Everything the proxy says on its begin stream, as the run scope's
/// own frame of the same meaning — a chunk as
/// [`Chunk`](response::Frame::Chunk), the loop's begin as
/// [`Active`](response::Frame::Active), its end as
/// [`Inactive`](response::Frame::Inactive) with the rotating secrets
/// the proxy collected, the values passed on unread — in order, until
/// the begin's stream ends — which is the proxy gone, not a loop
/// ended. A proxy that ends its stream with an error — a collect that
/// failed — is relayed as the run's [`Error`](response::Frame::Error),
/// in the proxy's words. The agents family's frame, named outright:
/// only an agent container has a conversation.
pub(crate) async fn chunks(run: Arc<Run>, mut chunks: Chunks) {
    while let Some(event) = chunks.next().await {
        let frame = match event {
            Ok(Event::Chunk(chunk)) => response::Frame::Chunk(chunk),
            Ok(Event::Active) => response::Frame::Active,
            Ok(Event::Inactive(values)) => response::Frame::Inactive(values),
            Err(ChunksError::Refused(error)) => response::Frame::Error(error),
            Err(_) => break,
        };
        send(&run.scope, encoded(&frame)).await;
    }
    run.over.notify_one();
}
