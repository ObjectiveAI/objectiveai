//! Running a tool container, from a scope and the provider's
//! capabilities.

use std::sync::Arc;

use super::Tools;
use crate::provider::endpoints::containers::server::handler;
use crate::provider::endpoints::containers::tools::run::client::request;
use crate::provider::server::container_deployer::ContainerDeployer;
use crate::provider::server::directory::Directory;
use crate::provider::server::image_registry::ImageRegistry;
use crate::wire::server::scope_handle::ScopeHandle;
use crate::provider::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Run the container the request describes and serve the scope for
/// the container's life.
///
/// The whole of it is `handler::run`, which both run families
/// share.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::provider::server::handle::handle) reads every
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
    handler::run::<Tools, D, G, V>(
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
