//! The tools a container declared, deployed by the caller.

use std::sync::Arc;

use super::send::{Stop, finish, respond};
use crate::client::ToolDeployer;
use crate::client::handle::Handle;
use crate::shared::containers::tools::{Tool, response};

/// One frame, deployed or the caller's refusal, then the finish.
pub(crate) async fn tools<T: ToolDeployer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    declared: Vec<Tool>,
    deployer: Arc<T>,
) -> Result<(), Stop> {
    let frame = match deployer.deploy(declared).await {
        Ok(()) => response::Frame::Deployed,
        Err(error) => response::Frame::Error(error),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}
