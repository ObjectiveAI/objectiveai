//! What every connection is served with.

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

use diverge_provider_sdk::connection::Connection;
use diverge_provider_sdk::server::authorization::Authorization;
use diverge_provider_sdk::server::directory::Directory;
use diverge_provider_sdk::server::handle::handle;
use diverge_provider_sdk::server::session::Session;

use super::Error;
use crate::config::Config;
use crate::config::auth::Auth;
use crate::config::volumes::Volumes;
use crate::container_deployer::ContainerDeployer;
use crate::image_checker::ImageChecker;
use crate::image_registry::ImageRegistry;
use crate::tools::podman;
use crate::unbrokered_authorizer::UnbrokeredAuthorizer;
use crate::volume_manager::VolumeManager;

/// The provider's pieces, built once and shared by every connection:
/// the SDK takes each behind an `Arc`, and the directory of running
/// containers is one per provider by the SDK's own rule, since a
/// connector on one connection names a container run on another.
#[derive(Debug)]
pub struct Provider {
    /// The `auth` section, from which an authorizer is made for each
    /// connection that dials in.
    auth: Option<Auth>,
    /// `<dir>/hooks/`, for that authorizer.
    hooks_dir: PathBuf,
    /// The deployer, held to the end so its tunnel lives as long.
    pub(super) deployer: Arc<ContainerDeployer>,
    /// The volumes.
    volumes: Arc<VolumeManager>,
    /// The image checker.
    checker: Arc<ImageChecker>,
    /// The registry, held to the end so its server lives as long.
    registry: Arc<ImageRegistry>,
    /// The running containers, one map for the provider.
    directory: Arc<Directory>,
}

impl Provider {
    /// Everything made, in the order its parts depend on each other:
    /// podman told where its data is; the registry started, since the
    /// deployer is told its address; the volumes; the image checker;
    /// the deployer, which brings the machine up, sweeps an earlier
    /// life away and opens the tunnel; and the directory.
    pub async fn start(config: Config, dir: PathBuf) -> Result<Self, Error> {
        podman::configure(config.containers.podman.storage_path.clone());
        let hooks_dir = dir.join("hooks");
        let registry = Arc::new(ImageRegistry::start().await.map_err(Error::Registry)?);
        let shares = config.volumes.as_ref().map(Volumes::paths).unwrap_or_default();
        let volumes = Arc::new(VolumeManager::new(config.volumes, hooks_dir.clone()));
        let checker = Arc::new(ImageChecker::new(&config.containers));
        let deployer = ContainerDeployer::new(config.containers, dir, Arc::clone(&volumes), registry.address(), shares)
            .await
            .map_err(Error::Deployer)?;
        Ok(Provider {
            auth: config.auth,
            hooks_dir,
            deployer: Arc::new(deployer),
            volumes,
            checker,
            registry,
            directory: Arc::new(Directory::new()),
        })
    }

    /// The authorization of a connection a peer dialled: the peer's
    /// first frame judged by an authorizer over the `auth` section,
    /// made here since the SDK takes one by value per connection and
    /// making one is a clone of the section.
    pub fn incoming(&self) -> Authorization<UnbrokeredAuthorizer> {
        Authorization::Incoming {
            unbrokered: UnbrokeredAuthorizer::new(self.auth.clone(), self.hooks_dir.clone()),
        }
    }

    /// One connection, served for as long as it lasts: the SDK's
    /// session over the socket and its `handle` with everything here.
    /// Why it ended is not kept — a peer that failed to authenticate
    /// and a peer that hung up are both a connection that is over,
    /// and there is nowhere to say which.
    pub async fn connection(&self, connection: Connection, authorization: Authorization<UnbrokeredAuthorizer>, address: IpAddr) {
        let session = Session::new(connection);
        let _ = handle(
            session,
            authorization,
            address,
            Arc::clone(&self.deployer),
            Arc::clone(&self.volumes),
            Arc::clone(&self.checker),
            Arc::clone(&self.registry),
            Arc::clone(&self.directory),
        )
        .await;
    }
}
