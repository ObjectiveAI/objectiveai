//! One of the container's server's tools, run.

use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::mcp::call_tool::{request, response};

use super::outcome;
use crate::encode::encoded;
use crate::proxy::Proxy;
use crate::reply::reply;

/// One exchange, entered and left as the container's activity: the
/// result or the error, as the container's server gave it and with
/// nothing set under `_meta`, then the finish. The error is sent as
/// the error, not folded into a result: the far side is the
/// provider's server, which does that folding itself.
pub async fn call_tool(proxy: &Proxy, scope: &ScopeHandle, channel: u32, request: request::Request) {
    proxy.activity.enter(scope).await;
    let frame = match proxy.tool.client().await {
        Err(error) => response::Frame::Error(error),
        Ok(client) => match outcome(client.call_tool(request.0).await) {
            Ok(result) => response::Frame::Result(result),
            Err(error) => response::Frame::Error(error),
        },
    };
    reply(scope, channel, encoded(&frame)).await;
    proxy.activity.leave(proxy, scope).await;
}
