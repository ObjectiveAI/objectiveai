//! The agent's conversation, from the begin's main stream onto the
//! run's.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::{Run, send};
use crate::container_proxy::outside::agents::begin::client::execute::Chunks;
use crate::provider::endpoints::containers::agents::run::server::response;

/// Every chunk the proxy sends, as the run scope's own
/// [`Chunk`](response::Frame::Chunk), in order, until the begin's
/// stream ends — which is the proxy gone. The agents family's frame,
/// named outright: only an agent container has a conversation.
pub(crate) async fn chunks(run: Arc<Run>, mut chunks: Chunks) {
    while let Some(Ok(chunk)) = chunks.next().await {
        send(&run.scope, encoded(&response::Frame::Chunk(chunk))).await;
    }
    run.over.notify_one();
}
