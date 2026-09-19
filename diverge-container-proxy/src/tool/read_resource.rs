//! One of the container's server's resources, read.

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::mcp::read_resource::{request, response};

use super::{Tool, outcome};
use crate::encode::encoded;
use crate::stamp::Stamp;
use crate::reply::reply;

/// One exchange: the result, the container's image under `_meta`, or
/// the error, then the finish.
pub async fn read_resource(tool: &Tool, stamp: &Stamp, scope: &ScopeHandle, channel: u32, request: request::Request) {
    let frame = match tool.client().await {
        Err(error) => response::Frame::Error(error),
        Ok(client) => match outcome(client.read_resource(request.0).await) {
            Ok(mut result) => {
                stamp.meta(&mut result.meta);
                response::Frame::Result(result)
            }
            Err(error) => response::Frame::Error(error),
        },
    };
    reply(scope, channel, encoded(&frame)).await;
}
