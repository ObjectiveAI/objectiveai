//! The container's MCP asks: the caller's servers, reached.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::run::Run;
use super::one;
use crate::container_proxy::mcp;
use crate::container_proxy::requests::execute::Ask;
use crate::decode::Decode as _;
use crate::server::answer::{Answer, answer};

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn list_tools<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| mcp::list_tools::response::Frame::decode(bytes).ok());
    let _ = mcp::list_tools::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn list_resources<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| mcp::list_resources::response::Frame::decode(bytes).ok());
    let _ = mcp::list_resources::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn call_tool<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| mcp::call_tool::response::Frame::decode(bytes).ok());
    let _ = mcp::call_tool::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn read_resource<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| mcp::read_resource::response::Frame::decode(bytes).ok());
    let _ = mcp::read_resource::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// The caller's notifications, for as long as it sends them, onto
/// the ask's path; the finish when the caller finishes. A caller that
/// goes away leaves the path unfinished, which the proxy reads as
/// re-ask — on a connection that is ending anyway.
pub(crate) async fn notifications<R: Runs>(run: Arc<Run>, ask: Ask) {
    let Some(payload) = ask.frame().ok().and_then(|frame| encoded(&R::relayed(frame.request)?)) else {
        return;
    };
    let mut channel = run.scope.send_channel_request(&payload).await;
    let Ok(mut handle) = mcp::notifications::execute::execute(&run.client, ask.channel).await else {
        return;
    };
    while let Some(bytes) = channel.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                let Ok(frame) = mcp::notifications::response::Frame::decode(&payload) else {
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
