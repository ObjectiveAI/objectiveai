//! Running an agent container, from a scope and the provider's
//! capabilities.

use std::sync::Arc;

use super::Agents;
use crate::endpoints::containers::agents::run::client::request;
use crate::endpoints::containers::server::handler;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::directory::Directory;
use crate::server::image_registry::ImageRegistry;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Run the container the request describes and serve the scope for
/// the container's life.
///
/// The whole of it is `handler::run`, which both run families
/// share.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle<D, G, V>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    deployer: &D,
    registry: &G,
    manager: &V,
    directory: Arc<Directory>,
) where
    D: ContainerDeployer,
    D::Error: Into<Error>,
    G: ImageRegistry,
    G::Error: Into<Error>,
    V: VolumeManager,
    V::Error: Into<Error>,
{
    handler::run::<Agents, D, G, V>(
        scope,
        client_identity,
        &request.0,
        request.0.arguments.clone(),
        deployer,
        registry,
        manager,
        directory,
    )
    .await
}
