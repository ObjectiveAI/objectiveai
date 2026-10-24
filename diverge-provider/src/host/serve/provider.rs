//! What every connection is served with.

use std::collections::HashSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, Either};
use tokio::sync::watch;

use diverge_sdk::shared::Identity;
use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::handshake::{self, Handshook};
use crate::protocol::acceptors::Acceptors;
use crate::protocol::directory::Directory;
use crate::protocol::changes::Changes;
use crate::protocol::handle::handle;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use super::capabilities::capabilities;
use diverge_sdk::wire::server::session::Session;

use crate::config::Config;
use crate::config::volumes::Volumes;

use super::{Error, Peers};
use crate::host::capacity_reporter::CapacityReporter;
use crate::host::container_deployer::ContainerDeployer;
use crate::host::image_checker::ImageChecker;
use crate::host::image_manager::{ImageManager, Store};
use crate::host::image_registry::ImageRegistry;
use crate::host::oci_store::OciStore;
use crate::host::tools::podman;
use crate::host::authenticator::ProviderAuthenticator;
use crate::host::volume_manager::{self, Scratch, VolumeManager};
use crate::host::Limit;

/// The provider's pieces, built once and shared by every connection:
/// the protocol takes each behind an `Arc`, and the directory of running
/// containers and the acceptors of daemon connections are one per
/// provider by the protocol's own rule, since a connector on one
/// connection names a container run, or a daemon accepting, on
/// another.
#[derive(Debug)]
pub struct Provider {
    /// The provider's side of every handshake: its own tokens, and the
    /// identities it serves.
    pub(super) authenticator: Arc<ProviderAuthenticator>,
    /// The deployer, held to the end so its tunnel lives as long.
    pub(super) deployer: Arc<ContainerDeployer>,
    /// The volumes.
    volumes: Arc<VolumeManager>,
    /// The image checker.
    checker: Arc<ImageChecker>,
    /// The client image store's manager.
    images: Arc<ImageManager>,
    /// What is held of an image for a caller: its client images, from
    /// its pool.
    oci: Arc<OciStore>,
    /// The registry, held to the end so its server lives as long.
    registry: Arc<ImageRegistry>,
    /// The running containers, one map for the provider.
    directory: Arc<Directory>,
    /// The daemons accepting connections from other daemons, one map
    /// for the provider.
    acceptors: Arc<Acceptors>,
    /// The peers connected now, one connection per identity.
    peers: Arc<Peers>,
    /// Every change to any caller's volumes, for the listings open.
    volume_changes: Arc<Changes>,
    /// Every change to any caller's images, for the listings open.
    image_changes: Arc<Changes>,
    /// What this provider holds out, answered by `info` and enforced
    /// by every handler: read off the configuration once.
    capabilities: Arc<Capabilities>,
    /// How much a caller may still use, answered by `capacity`: the
    /// stores and the deployer's caps, asked as of each answer.
    capacity: Arc<CapacityReporter>,
    /// The provider's stop: sent `true` by a loopback `POST /stop`
    /// (see [`stop`](super::stop)), or by the signal, and read by the
    /// listener, which drains on it, and by every connection, which
    /// closes on it. Made by [`run`](super::run) and held here so a
    /// handler can reach it.
    pub stop: watch::Sender<bool>,
}

impl Provider {
    /// Everything made, in the order its parts depend on each other:
    /// the provider's disk readied — `dir`, `<root>/provider/`, with
    /// `hooks/` and `run/`, podman's `storage_path` and every store
    /// made, every fixed volume found to be a directory, every store's
    /// capacity and every fixed name checked; podman told where its
    /// data is; the client image store, where the configuration
    /// names one, made first of all, since the registry serves its
    /// pools; the registry started, since the
    /// deployer is told its address; the container overlay cap and
    /// the scratch directory under podman's storage, swept; the
    /// volumes; the deployer, which
    /// brings the machine up, sweeps an earlier life away, writes the
    /// auth file and opens the tunnel; the image checker, given that
    /// auth file; the image manager, given the deployer; the capacity
    /// reporter, given the volumes, the store and the deployer; and
    /// the directory. The store is handed to the registry, the
    /// deployer, the checker, the OCI store, the reporter and the
    /// manager. `stop` is the caller's, made before anything here, so
    /// the signal and the stop route send on the one sender.
    /// `authenticator` is the provider's side of every handshake, made
    /// already.
    pub async fn start(config: Config, dir: PathBuf, stop: watch::Sender<bool>, authenticator: Arc<ProviderAuthenticator>) -> Result<Self, Error> {
        ready(&config, &dir).await?;
        let capabilities = Arc::new(capabilities(&config));
        podman::configure(config.containers.podman.storage_path.clone());
        let hooks_dir = dir.join("hooks");
        let mut shares = config.volumes.as_ref().map(Volumes::paths).unwrap_or_default();
        let store = config.images.as_ref().map(|images| {
            shares.push(images.path.clone());
            Arc::new(Store::new(images))
        });
        let registry = Arc::new(ImageRegistry::start(store.clone()).await.map_err(Error::Registry)?);
        let disk = Arc::new(Limit::new(config.containers.podman.container_overlay_disk));
        let scratch = Scratch::new(config.containers.podman.storage_path.join("ephemeral"), Arc::clone(&disk));
        scratch.sweep().await.map_err(Error::Scratch)?;
        let volumes = Arc::new(VolumeManager::new(config.volumes, hooks_dir.clone(), scratch));
        let registries = config.containers.podman.registries.clone();
        let deployer = Arc::new(
            ContainerDeployer::new(config.containers, dir, Arc::clone(&volumes), registry.address(), shares, disk, store.clone())
                .await
                .map_err(Error::Deployer)?,
        );
        let checker = Arc::new(ImageChecker::new(store.clone(), &registries, deployer.auth_file().to_path_buf()));
        let oci = Arc::new(OciStore::new(store.clone()));
        let capacity = Arc::new(CapacityReporter::new(Arc::clone(&volumes), store.clone(), Arc::clone(&deployer)));
        let images = Arc::new(ImageManager::new(store, Arc::clone(&volumes), Arc::clone(&deployer)));
        Ok(Provider {
            authenticator,
            deployer,
            volumes,
            checker,
            images,
            oci,
            registry,
            directory: Arc::new(Directory::new()),
            acceptors: Arc::new(Acceptors::new()),
            peers: Arc::new(Peers::default()),
            volume_changes: Arc::new(Changes::new()),
            image_changes: Arc::new(Changes::new()),
            capabilities,
            capacity,
            stop,
        })
    }

    /// A connection a peer dialled: its handshake as the connectee —
    /// see [`authenticator`](crate::host::authenticator) — and then the
    /// connection served under the peer's identity, which it holds for
    /// as long as it lasts. A handshake that fails, and an identity
    /// another connection holds, close the connection with nothing
    /// sent.
    pub async fn accept(&self, connection: Connection, address: IpAddr) {
        let Ok(Handshook { connection, peer }) = handshake::accept(connection, self.authenticator.as_ref(), address).await else {
            return;
        };
        self.held(connection, peer, address).await;
    }

    /// A connection past its handshake, under `peer`: the identity
    /// taken — a connection under an identity another holds is closed —
    /// the connection served, and the identity let go.
    pub async fn held(&self, connection: Connection, peer: Identity, address: IpAddr) {
        if !self.peers.take(&peer).await {
            return;
        }
        self.connection(connection, Arc::new(peer.clone()), address).await;
        self.peers.release(&peer).await;
    }

    /// One connection, served for as long as it lasts, or until the
    /// provider stops: the protocol's session over the socket and its
    /// `handle` with everything here, raced against `stop` turning
    /// `true`. Why it ended is not kept — a peer that broke the
    /// handshake's rule and a peer that hung up are both a connection
    /// that is over, and there is nowhere to say which.
    ///
    /// # The stop closes the socket
    ///
    /// The listener's drain waits only for HTTP requests in flight;
    /// an upgraded socket lives in its own task, and a dialled one in
    /// its dial's, and neither is the drain's to close. So the stop is
    /// read here, by every connection, and when it wins the race the
    /// `handle` in flight is dropped — the session with it, which is
    /// how the protocol ends a connection: every scope's feeds close,
    /// and the socket goes with the session. What the drop also takes
    /// is the handle's set of scopes, so a scope still being served
    /// is aborted rather than drained; the sweep that follows every
    /// stop removes every container of this provider by force, and the
    /// provider is dropped after it, so nothing a scope would have
    /// torn down outlives the stop.
    async fn connection(&self, connection: Connection, client_identity: Arc<Identity>, address: IpAddr) {
        let session = Session::new(connection);
        let mut stop = self.stop.subscribe();
        let serving = pin!(handle(
            session,
            client_identity,
            address,
            Arc::clone(&self.deployer),
            Arc::clone(&self.volumes),
            Arc::clone(&self.checker),
            Arc::clone(&self.images),
            Arc::clone(&self.oci),
            Arc::clone(&self.registry),
            Arc::clone(&self.capacity),
            Arc::clone(&self.directory),
            Arc::clone(&self.volume_changes),
            Arc::clone(&self.image_changes),
            Arc::clone(&self.acceptors),
            Arc::clone(&self.capabilities),
        ));
        let stopping = pin!(async move {
            let _ = stop.wait_for(|stopped| *stopped).await;
        });
        match future::select(serving, stopping).await {
            Either::Left((_, _)) => {}
            Either::Right(((), serving)) => drop(serving),
        }
    }
}

/// The provider's disk, as its block names it: `dir` with `hooks/`
/// and `run/` inside it, `storage_path` and every store made if
/// absent, the client image store's directory made if absent and
/// `run/builds/` swept; a fixed
/// volume whose path is not an existing directory refused, since a
/// fixed volume is content the operator already has; a store whose
/// capacity is `0` refused, and the image store's likewise; a fixed
/// volume whose name is not one a volume may have, or is another
/// fixed volume's, refused. What the file said is the SDK's to read; what it names
/// here is the provider's to check.
async fn ready(config: &Config, dir: &Path) -> Result<(), Error> {
    for made in [dir.join("hooks"), dir.join("run"), config.containers.podman.storage_path.clone()] {
        tokio::fs::create_dir_all(&made).await.map_err(|source| Error::Directory(made, source))?;
    }
    let builds = dir.join("run").join("builds");
    let _ = tokio::fs::remove_dir_all(&builds).await;
    tokio::fs::create_dir_all(&builds).await.map_err(|source| Error::Directory(builds, source))?;
    if let Some(images) = &config.images {
        if images.capacity == 0 {
            return Err(Error::ImageCapacity(images.path.clone()));
        }
        tokio::fs::create_dir_all(&images.path)
            .await
            .map_err(|source| Error::Directory(images.path.clone(), source))?;
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
