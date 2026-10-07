//! The agent's conversation, from the begin's main stream onto the
//! run's.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::{Run, send};
use crate::container_proxy::outside::endpoints::agents::begin::client::execute::Chunks;
use crate::provider::endpoints::containers::agents::run::client::execute::Event;
use crate::provider::endpoints::containers::agents::run::server::response;

/// Everything the proxy says on its begin stream, as the run scope's
/// own frame of the same meaning — a chunk as
/// [`Chunk`](response::Frame::Chunk), the loop's begin as
/// [`Active`](response::Frame::Active), its end as
/// [`Inactive`](response::Frame::Inactive) — in order, until the
/// begin's stream ends — which is the proxy gone, not a loop ended.
/// The agents family's frame, named outright: only an agent container
/// has a conversation.
pub(crate) async fn chunks(run: Arc<Run>, mut chunks: Chunks) {
    while let Some(Ok(event)) = chunks.next().await {
        let frame = match event {
            Event::Chunk(chunk) => response::Frame::Chunk(chunk),
            Event::Active => response::Frame::Active,
            Event::Inactive => response::Frame::Inactive,
        };
        send(&run.scope, encoded(&frame)).await;
    }
    run.over.notify_one();
}
