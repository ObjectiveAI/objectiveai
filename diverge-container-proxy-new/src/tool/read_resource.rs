//! One of the container's server's resources, read.

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::mcp::read_resource::{request, response};

use super::{Tool, outcome};
use crate::encode::encoded;
use crate::reply::reply;

/// One exchange: the result or the error, then the finish.
pub async fn read_resource(tool: &Tool, scope: &ScopeHandle, channel: u32, request: request::Request) {
    let frame = match tool.client().await {
        Err(error) => response::Frame::Error(error),
        Ok(client) => match outcome(client.read_resource(request.0).await) {
            Ok(result) => response::Frame::Result(result),
            Err(error) => response::Frame::Error(error),
        },
    };
    reply(scope, channel, encoded(&frame)).await;
}
