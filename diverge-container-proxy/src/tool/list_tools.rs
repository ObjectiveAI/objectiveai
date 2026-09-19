//! What tools the container's server offers.

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::mcp::list_tools::{request, response};

use super::{Tool, outcome};
use crate::encode::encoded;
use crate::stamp::Stamp;
use crate::reply::reply;

/// One exchange: the result — the container's image under `_meta`, on
/// it and on each tool — or the error, then the finish.
pub async fn list_tools(tool: &Tool, stamp: &Stamp, scope: &ScopeHandle, channel: u32, request: request::Request) {
    let frame = match tool.client().await {
        Err(error) => response::Frame::Error(error),
        Ok(client) => match outcome(client.list_tools(request.0).await) {
            Ok(mut result) => {
                stamp.meta(&mut result.meta);
                for tool in &mut result.tools {
                    stamp.tool(tool);
                }
                response::Frame::Result(result)
            }
            Err(error) => response::Frame::Error(error),
        },
    };
    reply(scope, channel, encoded(&frame)).await;
}
