//! One of the container's server's tools, run.

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::mcp::call_tool::{request, response};

use super::{Tool, outcome};
use crate::encode::encoded;
use crate::reply::reply;

/// One exchange: the result or the error, then the finish. The error
/// is sent as the error, not folded into a result: the far side is
/// the provider's server, which does that folding itself.
pub async fn call_tool(tool: &Tool, scope: &ScopeHandle, channel: u32, request: request::Request) {
    let frame = match tool.client().await {
        Err(error) => response::Frame::Error(error),
        Ok(client) => match outcome(client.call_tool(request.0).await) {
            Ok(result) => response::Frame::Result(result),
            Err(error) => response::Frame::Error(error),
        },
    };
    reply(scope, channel, encoded(&frame)).await;
}
