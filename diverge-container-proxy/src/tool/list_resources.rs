//! What resources the container's server offers.

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::mcp::list_resources::{request, response};

use super::{Tool, outcome};
use crate::encode::encoded;
use crate::reply::reply;

/// One exchange: the result or the error, then the finish.
pub async fn list_resources(tool: &Tool, scope: &ScopeHandle, channel: u32, request: request::Request) {
    let frame = match tool.client().await {
        Err(error) => response::Frame::Error(error),
        Ok(client) => match outcome(client.list_resources(request.0).await) {
            Ok(result) => response::Frame::Result(result),
            Err(error) => response::Frame::Error(error),
        },
    };
    reply(scope, channel, encoded(&frame)).await;
}
