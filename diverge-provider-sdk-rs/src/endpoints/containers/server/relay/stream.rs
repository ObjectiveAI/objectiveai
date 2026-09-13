//! An ask answered with a stream: a command's items, a notification
//! stream.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::run::Run;
use crate::container_proxy_endpoints::client::Ask;
use crate::server::answer::{Answer, answer};

/// Carry `ask` to the caller and put every answer on the proxy's
/// channel as it comes, then the finish when the caller finishes. A
/// caller that goes away leaves the proxy's channel unfinished — on a
/// connection that is ending anyway.
pub(crate) async fn stream<R: Runs>(run: Arc<Run>, channel: u32, ask: &Ask) {
    let Some(payload) = R::relayed(ask).and_then(|frame| encoded(&frame)) else {
        let _ = run.begin.finish(channel).await;
        return;
    };
    let mut caller = run.scope.send_channel_request(&payload).await;
    while let Some(bytes) = caller.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                if run.begin.respond(channel, &payload).await.is_err() {
                    return;
                }
            }
            Some(Answer::Finish) => {
                let _ = run.begin.finish(channel).await;
                return;
            }
            None => {}
        }
    }
}
