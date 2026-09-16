//! Running a tool container, from a scope and the provider's
//! capabilities.

use std::sync::Arc;

use super::Tools;
use crate::endpoints::containers::server::handler;
use crate::endpoints::containers::tools::run::client::request;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::identity_mount_manager::IdentityMountManager;
use crate::server::directory::Directory;
use crate::server::image_registry::ImageRegistry;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_mount_manager::VolumeMountManager;
use crate::shared::error::Error;

/// Run the container the request describes and serve the scope for
/// the container's life.
///
/// The whole of it is `handler::run`, which both run families
/// share; a tool container registers nothing, so nothing is added.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle<D, S, G, V>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    deployer: &D,
    store: &S,
    registry: &G,
    manager: &V,
    directory: Arc<Directory>,
) where
    D: ContainerDeployer,
    D::Error: Into<Error>,
    S: IdentityMountManager,
    S::Error: Into<Error>,
    G: ImageRegistry,
    G::Error: Into<Error>,
    V: VolumeMountManager,
    V::Error: Into<Error>,
{
    handler::run::<Tools, D, S, G, V>(scope, client_identity, &request.0, None, deployer, store, registry, manager, directory).await
}
