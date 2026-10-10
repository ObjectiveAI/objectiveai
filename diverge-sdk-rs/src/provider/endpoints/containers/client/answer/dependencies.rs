//! The dependencies a container declared, deployed by the caller.

use std::sync::Arc;

use super::send::{Stop, finish, respond};
use crate::provider::client::DependencyDeployer;
use crate::wire::client::handle::Handle;
use crate::shared::containers::dependencies::{Template, response};

/// One frame, deployed or the caller's refusal, then the finish.
pub(crate) async fn dependencies<T: DependencyDeployer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    id: String,
    declared: Vec<Template>,
    deployer: Arc<T>,
) -> Result<(), Stop> {
    let frame = match deployer.deploy(id, declared).await {
        Ok(()) => response::Frame::Deployed,
        Err(error) => response::Frame::Error(error),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}
