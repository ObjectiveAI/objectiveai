//! The container's commands: run by the caller, items streamed back.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::run::Run;
use crate::container_proxy::command;
use crate::container_proxy::requests::execute::Ask;
use crate::server::answer::{Answer, answer};

/// The caller's items, one message each, onto the ask's path; the
/// finish when the caller finishes, an `Error` last if it sent one.
/// A caller that goes away leaves the path unfinished, which the
/// proxy reports to the container as the command failed.
pub(crate) async fn command<R: Runs>(run: Arc<Run>, ask: Ask) {
    let Some(payload) = ask.frame().ok().and_then(|frame| encoded(&R::relayed(frame.request)?)) else {
        return;
    };
    let mut channel = run.scope.send_channel_request(&payload).await;
    let Ok(mut handle) = command::execute::execute(&run.client, ask.channel).await else {
        return;
    };
    while let Some(bytes) = channel.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                let Ok(frame) = command::response::Frame::decode(&payload) else {
                    continue;
                };
                if handle.send(&frame).await.is_err() {
                    return;
                }
            }
            Some(Answer::Finish) => {
                let _ = handle.finish().await;
                return;
            }
            None => {}
        }
    }
}
