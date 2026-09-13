//! The queue, cleared: the server's dequeue channel.

use std::sync::Arc;

use diverge_container_proxy_sdk::agent::dequeue::Outcome;
use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::containers::dequeue::response;
use reqwest::header::CONTENT_TYPE;
use tokio::sync::oneshot;

use super::{Cmd, refused, status_error};
use crate::encode::encoded;
use crate::proxy::Proxy;
use crate::reply::reply;

/// Serve one dequeue channel: the driver gives back every message
/// still waiting — each answered `Dequeued` on its own channel — and
/// says whether a loop runs; if one does, the agent's server is asked
/// to withdraw what IT holds, and a delivery on the wire answers for
/// itself. The answer is `Dequeued` when either side withdrew
/// anything, `Empty` when neither did, and `Error` when the agent's
/// server refused or could not be reached. Then the finish. A driver
/// that is gone, or one that never was, is the finish with nothing
/// before it.
///
/// One imprecision, accepted: a dequeue that drained nothing and
/// found the agent's server's queue empty answers `Empty`, though a
/// delivery on the wire may still come back missed and be answered
/// `Dequeued` — waiting on it would mean waiting on a call the loop
/// may hold for its life.
pub async fn dequeue(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32) {
    let (reply_sender, heard) = oneshot::channel();
    let sent = proxy
        .commands()
        .is_some_and(|commands| commands.send(Cmd::Dequeue(reply_sender)).is_ok());
    if !sent {
        reply(&scope, channel, None).await;
        return;
    }
    let Ok(replied) = heard.await else {
        reply(&scope, channel, None).await;
        return;
    };
    let frame = if replied.active {
        let response = proxy
            .upstream
            .http()
            .post(proxy.upstream.url("/dequeue"))
            .header(CONTENT_TYPE, "application/json")
            .body("{}")
            .send()
            .await;
        match response {
            Err(error) => response::Frame::Error(refused(&error)),
            Ok(response) if !response.status().is_success() => response::Frame::Error(status_error(response).await),
            Ok(response) => match response.json::<Outcome>().await {
                Ok(Outcome::Dequeued) => response::Frame::Dequeued,
                Ok(Outcome::Empty) if replied.drained > 0 => response::Frame::Dequeued,
                Ok(Outcome::Empty) => response::Frame::Empty,
                Err(error) => response::Frame::Error(diverge_provider_sdk::shared::error::Error(serde_json::json!({
                    "kind": "agent",
                    "error": format!("the outcome did not parse: {error}"),
                }))),
            },
        }
    } else if replied.drained > 0 {
        response::Frame::Dequeued
    } else {
        response::Frame::Empty
    };
    reply(&scope, channel, encoded(&frame)).await;
}
