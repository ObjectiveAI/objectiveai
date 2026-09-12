//! An ask answered with one frame.

use bytes::Bytes;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::run::Run;
use crate::container_proxy::requests::execute::Ask;
use crate::server::answer::{Answer, answer};

/// Carry `ask` to the caller as the family's frame and wait for its
/// one answer: the payload, or `None` for a finish with nothing in
/// front — could not serve — or a caller that is gone. The channel is
/// read to its finish either way, so its number comes back.
pub(crate) async fn ask<R: Runs>(run: &Run, ask: &Ask) -> Option<Bytes> {
    let payload = {
        let frame = ask.frame().ok()?;
        encoded(&R::relayed(frame.request)?)?
    };
    let mut channel = run.scope.send_channel_request(&payload).await;
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
