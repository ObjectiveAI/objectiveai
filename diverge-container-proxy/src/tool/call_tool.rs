//! One of the container's server's tools, run.

use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::mcp::call_tool::{request, response};

use super::{Tool, outcome};
use crate::encode::encoded;
use crate::stamp::Stamp;
use crate::reply::reply;

/// One exchange: the result, the container's image under `_meta`, or
/// the error, then the finish. The error
/// is sent as the error, not folded into a result: the far side is
/// the provider's server, which does that folding itself.
pub async fn call_tool(tool: &Tool, stamp: &Stamp, scope: &ScopeHandle, channel: u32, request: request::Request) {
    let frame = match tool.client().await {
        Err(error) => response::Frame::Error(error),
        Ok(client) => match outcome(client.call_tool(request.0).await) {
            Ok(mut result) => {
                stamp.meta(&mut result.meta);
                response::Frame::Result(result)
            }
            Err(error) => response::Frame::Error(error),
        },
    };
    reply(scope, channel, encoded(&frame)).await;
}
