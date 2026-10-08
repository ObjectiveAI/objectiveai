//! What every connection is served with.

use std::collections::HashSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;

use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::server::authorization::Authorization;
use diverge_sdk::provider::server::directory::Directory;
use diverge_sdk::provider::server::volume_changes::VolumeChanges;
use diverge_sdk::provider::server::handle::handle;
use diverge_sdk::wire::server::session::Session;

use diverge_sdk::config::provider::Config;
use diverge_sdk::config::provider::auth::Auth;
use diverge_sdk::config::provider::volumes::Volumes;

use super::{Error, Peers};
use crate::container_deployer::ContainerDeployer;
use crate::image_checker::ImageChecker;
use crate::image_registry::ImageRegistry;
use crate::tools::podman;
use crate::unbrokered_authorizer::{Slot, UnbrokeredAuthorizer};
use crate::volume_manager::{self, Scratch, VolumeManager};
use crate::Limit;

/// The provider's pieces, built once and shared by every connection:
/// the SDK takes each behind an `Arc`, and the directory of running
/// containers is one per provider by the SDK's own rule, since a
/// connector on one connection names a container run on another.
#[derive(Debug)]
pub struct Provider {
    /// The `auth` section, from which an authorizer is made for each
    /// connection that dials in.
    auth: Option<Auth>,
    /// `<root>/provider/hooks/`, for that authorizer.
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
    /// The peers connected now, one per identity and per credential.
    peers: Arc<Peers>,
    /// Every change to any caller's volumes, for the listings open.
    volume_changes: Arc<VolumeChanges>,
}

impl Provider {
    /// Everything made, in the order its parts depend on each other:
    /// the provider's disk readied — `dir`, `<root>/provider/`, with
    /// `hooks/` and `run/`, podman's `storage_path` and every store
    /// made, every fixed volume found to be a directory, every store's
    /// capacity and every fixed name checked; podman told where its
    /// data is; the registry started, since the
    /// deployer is told its address; the container overlay cap and
    /// the scratch directory under podman's storage, swept; the
    /// volumes; the deployer, which
    /// brings the machine up, sweeps an earlier life away, writes the
    /// auth file and opens the tunnel; the image checker, given that
    /// auth file; and the directory.
    pub async fn start(config: Config, dir: PathBuf) -> Result<Self, Error> {
        ready(&config, &dir).await?;
        podman::configure(config.containers.podman.storage_path.clone());
        let hooks_dir = dir.join("hooks");
        let registry = Arc::new(ImageRegistry::start().await.map_err(Error::Registry)?);
        let shares = config.volumes.as_ref().map(Volumes::paths).unwrap_or_default();
        let disk = Arc::new(Limit::new(config.containers.podman.container_overlay_disk));
        let scratch = Scratch::new(config.containers.podman.storage_path.join("ephemeral"), Arc::clone(&disk));
        scratch.sweep().await.map_err(Error::Scratch)?;
        let volumes = Arc::new(VolumeManager::new(config.volumes, hooks_dir.clone(), scratch));
        let images = config.containers.server_images.clone();
        let registries = config.containers.podman.registries.clone();
        let deployer = ContainerDeployer::new(config.containers, dir, Arc::clone(&volumes), registry.address(), shares, disk)
            .await
            .map_err(Error::Deployer)?;
        let checker = Arc::new(ImageChecker::new(&images, &registries, deployer.auth_file().to_path_buf()));
        Ok(Provider {
            auth: config.auth,
            hooks_dir,
            deployer: Arc::new(deployer),
            volumes,
            checker,
            registry,
            directory: Arc::new(Directory::new()),
            peers: Arc::new(Peers::default()),
            volume_changes: Arc::new(VolumeChanges::new()),
        })
    }

    /// A connection a peer dialled, served for as long as it lasts:
    /// the peer's first frame judged by an authorizer over the `auth`
    /// section — made here since the SDK takes one by value per
    /// connection and making one is a clone of the section — which
    /// takes the identity and the credential among the peers
    /// connected, and refuses a second connection on either; when the
    /// connection is over, whatever it took is given back.
    pub async fn accept(&self, connection: Connection, address: IpAddr) {
        let taken: Slot = Arc::new(Mutex::new(None));
        let authorization = Authorization::Incoming {
            unbrokered: UnbrokeredAuthorizer::new(self.auth.clone(), self.hooks_dir.clone(), Arc::clone(&self.peers), Arc::clone(&taken)),
        };
        self.connection(connection, authorization, address).await;
        if let Some(held) = taken.lock().await.take() {
            self.peers.release(&held.identity, &held.credential).await;
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
            Arc::clone(&self.volume_changes),
        )
        .await;
    }
}

/// The provider's disk, as its block names it: `dir` with `hooks/`
/// and `run/` inside it, `storage_path` and every store made if
/// absent; a fixed volume whose path is not an existing directory
/// refused, since a fixed volume is content the operator already has;
/// a store whose capacity is `0` refused; a fixed volume whose name
/// is not one a volume may have, or is another fixed volume's,
/// refused. What the file said is the SDK's to read; what it names
/// here is the provider's to check.
async fn ready(config: &Config, dir: &Path) -> Result<(), Error> {
    for made in [dir.join("hooks"), dir.join("run"), config.containers.podman.storage_path.clone()] {
        tokio::fs::create_dir_all(&made).await.map_err(|source| Error::Directory(made, source))?;
    }
    let Some(volumes) = &config.volumes else {
        return Ok(());
    };
    for store in volumes.stores.iter().flatten() {
        if store.capacity == 0 {
            return Err(Error::Capacity(store.path.clone()));
        }
        tokio::fs::create_dir_all(&store.path)
            .await
            .map_err(|source| Error::Directory(store.path.clone(), source))?;
    }
    let mut names = HashSet::new();
    for fixed in volumes.fixed.iter().flatten() {
        if !tokio::fs::metadata(&fixed.path).await.is_ok_and(|found| found.is_dir()) {
            return Err(Error::FixedMissing(fixed.path.clone()));
        }
        if !volume_manager::ok(&fixed.name) || !names.insert(fixed.name.as_str()) {
            return Err(Error::FixedName(fixed.name.clone()));
        }
    }
    Ok(())
}
