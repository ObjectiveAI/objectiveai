//! Running an agent container, from a scope and the provider's
//! capabilities.

use super::Agents;
use crate::endpoints::containers::agents::run::client::request;
use crate::endpoints::containers::server::handler;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::content_store::ContentStore;
use crate::server::directory::Directory;
use crate::server::image_registry::ImageRegistry;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Run the container the request describes, register its agent, and
/// serve the scope for the container's life.
///
/// The whole of it is `handler::run`, which both run families
/// share; what this family adds is the agent value, registered with
/// the container's own server before the id goes out, so the first
/// loop the caller asks for runs as that agent.
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
    directory: &Directory,
) where
    D: ContainerDeployer,
    D::Error: Into<Error>,
    S: ContentStore,
    S::Error: Into<Error>,
    G: ImageRegistry,
    G::Error: Into<Error>,
    V: VolumeManager,
    V::Error: Into<Error>,
{
    handler::run::<Agents, D, S, G, V>(
        scope,
        client_identity,
        &request.container,
        Some(request.agent),
        deployer,
        store,
        registry,
        manager,
        directory,
    )
    .await
}
