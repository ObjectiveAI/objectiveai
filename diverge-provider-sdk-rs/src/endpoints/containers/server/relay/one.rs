//! An ask answered once.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::run::Run;
use crate::container_proxy_endpoints::client::Ask;
use crate::server::answer::{Answer, answer};

/// Carry `ask` to the caller as the family's frame, wait for its one
/// answer, and put it on the proxy's channel: the answer, then the
/// finish — or the finish alone, for a finish with nothing in front,
/// the caller's could-not-serve, or a caller that is gone. The
/// caller's channel is read to its finish either way, so its number
/// comes back.
pub(crate) async fn one<R: Runs>(run: Arc<Run>, channel: u32, ask: &Ask) {
    let payload = R::relayed(ask).and_then(|frame| encoded(&frame));
    let first = match payload {
        Some(payload) => first(&run, &payload).await,
        None => None,
    };
    if let Some(bytes) = first {
        let _ = run.begin.respond(channel, &bytes).await;
    }
    let _ = run.begin.finish(channel).await;
}

/// Open the channel and wait for its first answer, reading to the
/// finish.
async fn first(run: &Run, payload: &[u8]) -> Option<bytes::Bytes> {
    let mut channel = run.scope.send_channel_request(payload).await;
    let mut first = None;
    while let Some(bytes) = channel.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                if first.is_none() {
                    first = Some(payload);
                }
            }
            Some(Answer::Finish) => break,
            None => {}
        }
    }
    first
}
