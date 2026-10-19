//! What resources the container's server offers.

use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::mcp::list_resources::{request, response};

use super::outcome;
use crate::encode::encoded;
use crate::proxy::Proxy;
use crate::reply::reply;

/// One exchange, entered and left as the container's activity: the
/// result — the container's image under `_meta`, on it and on each
/// resource — or the error, then the finish.
pub async fn list_resources(proxy: &Proxy, scope: &ScopeHandle, channel: u32, request: request::Request) {
    proxy.activity.enter(scope).await;
    let frame = match proxy.tool.client().await {
        Err(error) => response::Frame::Error(error),
        Ok(client) => match outcome(client.list_resources(request.0).await) {
            Ok(result) => response::Frame::Result(result),
            Err(error) => response::Frame::Error(error),
        },
    };
    reply(scope, channel, encoded(&frame)).await;
    proxy.activity.leave(proxy, scope).await;
}
