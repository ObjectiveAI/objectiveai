//! One run's mounts, built and asked.

use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::server::channel_request::Frame as Ask;
use diverge_sdk::daemon::endpoints::agents::create::client::request::FuseMount as VolumeFuseMount;
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::daemon::endpoints::volumes::fuse_serve::client::execute as daemon_serve;
use diverge_sdk::daemon::endpoints::volumes::fuse_serve::client::request::Frame as DaemonServeRequest;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::client::Listed;
use diverge_sdk::provider::endpoints::containers::fuse_serve::client::execute::{self as container_serve, ExecuteHandle as ContainerServe};
use diverge_sdk::provider::endpoints::containers::fuse_serve::client::request::Frame as ContainerServeRequest;
use diverge_sdk::provider::endpoints::volumes::fuse_serve::client::execute::{self as volume_serve, ExecuteHandle as VolumeServe};
use diverge_sdk::provider::endpoints::volumes::fuse_serve::client::request::Frame as VolumeServeRequest;
use diverge_sdk::shared::containers::dependencies::Mount;
use diverge_sdk::shared::containers::fuse::Attrs;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::containers::fuse::stat::Stat;
use diverge_sdk::shared::containers::request::FuseMount;
use diverge_sdk::shared::filetree::response::Frame;
use diverge_sdk::shared::relay::Relay;
use diverge_sdk::wire::client::channel::Channel;
use tokio::sync::{Mutex, RwLock, broadcast, watch as signal};
use tokio::task::AbortHandle;

use super::{bridge, watch};
use crate::containers::capable;
use crate::daemon::{Capacity, Daemon};
use crate::daemons;

/// The scope a mount's bytes come from: a volume served by its
/// provider, a volume served by another daemon, or a subtree of a
/// running container served by the provider it runs on. All answer
/// the same nine asks in the same frames, so a mount is asked without
/// knowing which.
pub enum Serve {
    /// A volume, served: a provider's `volumes::fuse_serve`, or
    /// another daemon's `volumes fuse_serve` — one handle for both,
    /// since the two scopes share their channels byte for byte. Shared
    /// with the task that waits for its end: see [`Mounts::lost`].
    Volume(Arc<VolumeServe>),
    /// A `containers::fuse_serve`: an agent's path, into its dependency.
    Container(ContainerServe),
}

impl Serve {
    /// One ask, and its one answer, as the scope's frames carry them.
    pub async fn ask(&self, ask: &Ask<'_>) -> Result<Bytes, String> {
        match self {
            Serve::Volume(serve) => serve.ask(ask).await.map_err(|error| format!("{error:?}")),
            Serve::Container(serve) => serve.ask(ask).await.map_err(|error| format!("{error:?}")),
        }
    }

    /// The tree as served, and its changes.
    pub async fn filetree(&self) -> Result<Channel, String> {
        match self {
            Serve::Volume(serve) => serve.filetree().await.map_err(|error| format!("{error:?}")),
            Serve::Container(serve) => serve.filetree().await.map_err(|error| format!("{error:?}")),
        }
    }

    /// The scope stopped.
    pub async fn stop(self) {
        match self {
            Serve::Volume(serve) => serve.cancel().await,
            Serve::Container(serve) => serve.cancel().await,
        }
    }
}

/// Why a run's mounts could not be built: a provider that holds out
/// no capability for what a mount asks, named, or a serve that could
/// not be opened, in a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MountsError {
    /// A provider's capabilities exclude the serve a mount needs: the
    /// volume's provider FUSE serves no volume of the mode.
    Unsupported(String),
    /// A provider not connected, or a serve it refused or failed.
    Failed(String),
}

impl std::fmt::Display for MountsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MountsError::Unsupported(what) | MountsError::Failed(what) => f.write_str(what),
        }
    }
}

/// Where one mount's bytes are: a serve of some provider's, to the
/// daemon; every ask is forwarded under the mount's path within it.
pub struct Served {
    /// The serve scope, until it is stopped.
    pub serve: Mutex<Option<Serve>>,
    /// The path within the served tree the mount is at: a path in
    /// the volume, or — a dependency's mount — empty for a directory
    /// served whole and the file's name for a file served with its
    /// directory.
    pub prefix: Vec<String>,
    /// The serving provider's capacity cell, where the serve took
    /// room there — a volume serve reserves its `overlay_disk` — so
    /// that the stop can nudge it without the daemon in hand; `None`
    /// for a container serve, which reserves nothing.
    pub capacity: Option<Arc<Capacity>>,
}

impl Served {
    /// The serve stopped, if it was not already, and the provider
    /// nudged to say its capacity again, the serve's room given back.
    async fn stop(&self) {
        if let Some(serve) = self.serve.lock().await.take() {
            serve.stop().await;
            if let Some(cell) = &self.capacity {
                cell.nudge();
            }
        }
    }
}

/// The mounts of one run: what the provider is told to mount, and
/// where each is answered from — behind one lock, since a volume
/// mount can be added while the run lives, see [`add`](Self::add).
pub struct Mounts {
    table: RwLock<Table>,
    /// Whether a volume mount's serve ended before the run let it go:
    /// see [`lost`](Self::lost).
    lost: signal::Sender<bool>,
    /// Every change made under a mount, as a frame from the
    /// container's root, for whoever watches the container.
    pub(super) changes: broadcast::Sender<Frame>,
    /// The watches of the served trees, for the mounts whose source
    /// streams its own changes: ended with the mounts.
    watchers: Mutex<Vec<AbortHandle>>,
}

/// What the lock protects: every mount's source by id, the two lists
/// as the provider is told them, the number the next volume mount's
/// id is minted from, and whether the run is over.
struct Table {
    sources: HashMap<String, Arc<Served>>,
    files: Vec<FuseMount>,
    directories: Vec<FuseMount>,
    next: usize,
    stopped: bool,
}

impl Table {
    /// The next volume mount's id: `v1`, `v2`, … for the run's life,
    /// never one given before. A dependency's mounts are `d1`, `d2`,
    /// … in a table of their own, which nothing is added to.
    fn mint(&mut self) -> String {
        self.next += 1;
        format!("v{}", self.next)
    }

    /// Keep a served mount under `id` at `container_path`, on the
    /// list its kind names: what the provider is told of it.
    fn insert(&mut self, id: String, served: Served, container_path: Vec<String>, directory: bool) -> FuseMount {
        self.sources.insert(id.clone(), Arc::new(served));
        let fuse = FuseMount { container_path, id };
        if directory {
            self.directories.push(fuse.clone());
        } else {
            self.files.push(fuse.clone());
        }
        fuse
    }

    /// The source the ask names, if the id is one of this run's.
    fn source(&self, id: &str) -> Result<Arc<Served>, String> {
        self.sources.get(id).cloned().ok_or_else(|| format!("no mount is served as {id}"))
    }

    /// Every mount, the files first.
    fn all(&self) -> impl Iterator<Item = &FuseMount> {
        self.files.iter().chain(&self.directories)
    }
}

impl Mounts {
    /// No mount: a connected tool's, whose container is somebody
    /// else's.
    pub fn empty() -> Mounts {
        Mounts {
            table: RwLock::new(Table {
                sources: HashMap::new(),
                files: Vec::new(),
                directories: Vec::new(),
                next: 0,
                stopped: false,
            }),
            lost: signal::channel(false).0,
            changes: broadcast::channel(256).0,
            watchers: Mutex::new(Vec::new()),
        }
    }

    /// The word that a mount of the run's lost its serve: `true` once
    /// the serve of any volume mount ended while the run still held
    /// it. A mount is a promise that the container can read and write
    /// what it mounted, and a serve that ends — the volume's provider
    /// gone, or the connection to the volume's daemon gone, at any hop
    /// — breaks it for good: nothing reopens a serve under a running
    /// container. Whoever runs the container ends the run on this
    /// word. The rules:
    ///
    /// 1. **Every volume mount is waited on**, a volume of this
    ///    daemon's and of another's alike, from the moment it is kept.
    ///    A dependency's mount, served from its agent's container, is
    ///    not: that container's end is the dependency's own end.
    /// 2. **Let go is not lost.** A serve the run stopped itself — at
    ///    its end, or for a mount the provider did not make — ends by
    ///    the run's own cancel, and says nothing here: the serve is
    ///    taken out of its slot before it is cancelled, and a serve
    ///    whose slot is empty when it ends was let go.
    pub fn lost(&self) -> signal::Receiver<bool> {
        self.lost.subscribe()
    }

    /// Rule 1 of [`lost`](Self::lost) for the mount under `id`: its
    /// serve waited on, in a task ended with the mounts.
    async fn await_loss(&self, id: &str) {
        let Some(served) = self.table.read().await.sources.get(id).cloned() else {
            return;
        };
        let serve = match &*served.serve.lock().await {
            Some(Serve::Volume(serve)) => Arc::clone(serve),
            _ => return,
        };
        let lost = self.lost.clone();
        let waiting = tokio::spawn(async move {
            serve.wait().await;
            // Rule 2.
            if served.serve.lock().await.is_some() {
                lost.send_replace(true);
            }
        });
        self.watchers.lock().await.push(waiting.abort_handle());
    }

    /// Every mount, the files first, as the provider is told them.
    pub(super) async fn all(&self) -> Vec<FuseMount> {
        self.table.read().await.all().cloned().collect()
    }

    /// The mount under `id`, if one.
    pub(super) async fn find(&self, id: &str) -> Option<FuseMount> {
        self.table.read().await.all().find(|mount| mount.id == id).cloned()
    }

    /// The two lists as the provider is told them: the file mounts,
    /// and the directory mounts.
    pub async fn lists(&self) -> (Vec<FuseMount>, Vec<FuseMount>) {
        let table = self.table.read().await;
        (table.files.clone(), table.directories.clone())
    }

    /// The mounts for a record's run: its cross-provider mounts, every
    /// volume's FUSE serve opened on its provider — a provider whose
    /// capabilities FUSE serve a volume of the mount's mode, else the
    /// mount is unsupported and the provider is not asked — and each
    /// such provider nudged to say its capacity again, the serve
    /// having taken its `overlay_disk`. The first that cannot be is
    /// the failure, and what was opened is let go.
    pub async fn build(daemon: &Arc<Daemon>, volume_files: &[VolumeFuseMount], volume_directories: &[VolumeFuseMount]) -> Result<Mounts, MountsError> {
        let mounts = Mounts::empty();
        for (mount, directory) in volume_files
            .iter()
            .map(|mount| (mount, false))
            .chain(volume_directories.iter().map(|mount| (mount, true)))
        {
            let served = match serve_volume(daemon, mount).await {
                Ok(served) => served,
                Err(error) => {
                    mounts.stop().await;
                    return Err(error);
                }
            };
            let id = {
                let mut table = mounts.table.write().await;
                let id = table.mint();
                table.insert(id.clone(), served, mount.container_path.clone(), directory);
                id
            };
            mounts.await_loss(&id).await;
        }
        Ok(mounts)
    }

    /// One more volume mount while the run lives, served as `build`
    /// serves each, and kept under the next id, on the list its kind
    /// names: what the provider is to be told of it. Nothing is told
    /// here — not the provider, not whoever watches the container:
    /// the caller asks the provider to mount it, and then either
    /// [`announce`](Self::announce)s it or
    /// [`drop_mount`](Self::drop_mount)s it. A run that is over takes
    /// no mount, and the serve opened for one is let go.
    pub async fn add(&self, daemon: &Arc<Daemon>, mount: &VolumeFuseMount, directory: bool) -> Result<FuseMount, MountsError> {
        let served = serve_volume(daemon, mount).await?;
        let fuse = {
            let mut table = self.table.write().await;
            if table.stopped {
                drop(table);
                served.stop().await;
                return Err(MountsError::Failed("the run is over".to_string()));
            }
            let id = table.mint();
            table.insert(id, served, mount.container_path.clone(), directory)
        };
        self.await_loss(&fuse.id).await;
        Ok(fuse)
    }

    /// The mount under `id` let go: the provider did not mount it.
    /// Its serve is stopped, and the provider that served it nudged.
    pub async fn drop_mount(&self, id: &str) {
        let served = {
            let mut table = self.table.write().await;
            table.files.retain(|mount| mount.id != id);
            table.directories.retain(|mount| mount.id != id);
            table.sources.remove(id)
        };
        if let Some(served) = served {
            served.stop().await;
        }
    }

    /// The mount under `id` is made: told to whoever watches the
    /// container, as the mount point appearing whole.
    pub async fn announce(&self, id: &str) {
        self.changed(id, "", true).await;
    }

    /// The mounts for a transient dependency's run: the agent's paths
    /// the template names, `files` and `directories`, each served by
    /// the agent's provider from the agent's container — a directory
    /// whole, a file with the directory holding it — and watched for
    /// every change, the agent's own included. A container serve reserves no
    /// room, so no capacity is asked again for it. The first that
    /// cannot be served is the failure, and what was opened is let go.
    pub async fn build_dependency(daemon: &Daemon, agent_provider: &Identity, agent_container: &str, files: &[Mount], directories: &[Mount]) -> Result<Mounts, MountsError> {
        let mounts = Mounts::empty();
        if files.is_empty() && directories.is_empty() {
            return Ok(mounts);
        }
        let Some(handle) = daemon.live.provider(agent_provider).await else {
            return Err(MountsError::Failed("the agent's provider is not connected now".to_string()));
        };
        let mut d = 0usize;
        for (mount, directory) in files
            .iter()
            .map(|mount| (mount, false))
            .chain(directories.iter().map(|mount| (mount, true)))
        {
            d += 1;
            let id = format!("d{d}");
            // A file is served with the directory holding it, since a
            // serve names a directory; the mount's prefix is then the
            // file's name.
            let (path, prefix) = if directory {
                (mount.agent_path.clone(), Vec::new())
            } else {
                let mut path = mount.agent_path.clone();
                let Some(name) = path.pop() else {
                    mounts.stop().await;
                    return Err(MountsError::Failed("a file mount names the agent's root".to_string()));
                };
                (path, vec![name])
            };
            let request = ContainerServeRequest {
                id: agent_container.to_string(),
                path,
            };
            let serve = match container_serve::execute(&handle, &request).await {
                Ok(serve) => serve,
                Err(error) => {
                    mounts.stop().await;
                    return Err(MountsError::Failed(format!("the agent's path {} could not be served: {error:?}", mount.agent_path.join("/"))));
                }
            };
            let tree = match serve.filetree().await {
                Ok(tree) => tree,
                Err(error) => {
                    serve.cancel().await;
                    mounts.stop().await;
                    return Err(MountsError::Failed(format!("the agent's path {} could not be watched: {error:?}", mount.agent_path.join("/"))));
                }
            };
            let watcher = tokio::spawn(watch::relay(tree, mounts.changes.clone(), mount.tool_path.clone(), prefix.clone()));
            mounts.watchers.lock().await.push(watcher.abort_handle());
            let served = Served {
                serve: Mutex::new(Some(Serve::Container(serve))),
                prefix,
                capacity: None,
            };
            mounts.table.write().await.insert(id, served, mount.tool_path.clone(), directory);
        }
        Ok(mounts)
    }

    /// The run is over: every watch ended, every serve stopped, and
    /// every provider that served a volume nudged to say its capacity
    /// again, the serve's room given back. Nothing is added after.
    pub async fn stop(&self) {
        for watcher in self.watchers.lock().await.drain(..) {
            watcher.abort();
        }
        let sources: Vec<Arc<Served>> = {
            let mut table = self.table.write().await;
            table.stopped = true;
            table.sources.drain().map(|(_, served)| served).collect()
        };
        for served in sources {
            served.stop().await;
        }
    }

    /// The source the ask names, if the id is one of this run's.
    async fn source(&self, id: &str) -> Result<Arc<Served>, String> {
        self.table.read().await.source(id)
    }

    /// What is at the path, if anything.
    pub async fn stat(&self, id: &str, path: &str) -> Result<Option<Stat>, String> {
        let served = self.source(id).await?;
        bridge::stat(&served.serve, &served.prefix, path).await
    }

    /// A piece of the file at the path.
    pub async fn read(&self, id: &str, path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
        let served = self.source(id).await?;
        bridge::read(&served.serve, &served.prefix, path, offset, length).await
    }

    /// A piece written into the file at the path.
    pub async fn write(&self, id: &str, path: &str, offset: u64, bytes: Bytes) -> Result<(), Refused> {
        let served = self.source(id).await.map_err(Refused::Error)?;
        bridge::write(&served.serve, &served.prefix, path, offset, &bytes).await?;
        if !streams(&served).await {
            self.changed(id, path, false).await;
        }
        Ok(())
    }

    /// The file at the path made `size` long.
    pub async fn truncate(&self, id: &str, path: &str, size: u64) -> Result<(), Refused> {
        let served = self.source(id).await.map_err(Refused::Error)?;
        bridge::truncate(&served.serve, &served.prefix, path, size).await?;
        if !streams(&served).await {
            self.changed(id, path, false).await;
        }
        Ok(())
    }

    /// The attributes of what is at the path changed.
    pub async fn setattr(&self, id: &str, path: &str, attrs: Attrs) -> Result<(), Refused> {
        let served = self.source(id).await.map_err(Refused::Error)?;
        bridge::setattr(&served.serve, &served.prefix, path, attrs).await
    }

    /// The entries of the directory at the path.
    pub async fn list(&self, id: &str, path: &str) -> Result<Option<Vec<Listed>>, String> {
        let served = self.source(id).await?;
        bridge::list(&served.serve, &served.prefix, path).await
    }

    /// What is at the path removed.
    pub async fn remove(&self, id: &str, path: &str) -> Result<(), Refused> {
        let served = self.source(id).await.map_err(Refused::Error)?;
        bridge::remove(&served.serve, &served.prefix, path).await?;
        if !streams(&served).await {
            self.removed(id, path).await;
        }
        Ok(())
    }

    /// What is at `from` moved to `to`.
    pub async fn rename(&self, id: &str, from: &str, to: &str) -> Result<(), Refused> {
        let served = self.source(id).await.map_err(Refused::Error)?;
        bridge::rename(&served.serve, &served.prefix, from, to).await?;
        if !streams(&served).await {
            self.removed(id, from).await;
            self.changed(id, to, true).await;
        }
        Ok(())
    }

    /// A directory made at the path.
    pub async fn mkdir(&self, id: &str, path: &str) -> Result<(), Refused> {
        let served = self.source(id).await.map_err(Refused::Error)?;
        bridge::mkdir(&served.serve, &served.prefix, path).await?;
        if !streams(&served).await {
            self.changed(id, path, true).await;
        }
        Ok(())
    }
}

/// Whether the mount's source streams its own changes, so that the
/// daemon need not report the ones it makes.
async fn streams(served: &Served) -> bool {
    matches!(&*served.serve.lock().await, Some(Serve::Container(_)))
}

/// One volume mount's serve, opened where the volume is. The rules:
///
/// 1. **A volume of this daemon's** is served by its provider — a
///    provider whose capabilities FUSE serve a volume of the mount's
///    mode, else the mount is unsupported and the provider is not
///    asked — and the provider nudged to say its capacity again, the
///    serve having taken its `overlay_disk`.
/// 2. **A volume on another daemon** is served by that daemon: see
///    [`serve_remote`].
async fn serve_volume(daemon: &Arc<Daemon>, mount: &VolumeFuseMount) -> Result<Served, MountsError> {
    if let Some(id) = mount.daemon {
        return serve_remote(daemon, mount, id).await;
    }
    let Some(handle) = daemon.live.provider(&mount.provider).await else {
        return Err(MountsError::Failed(format!("the provider of the volume {} is not connected now", mount.volume_name)));
    };
    let lacked = daemon
        .live
        .capabilities(&mount.provider)
        .await
        .and_then(|info| capable::fuse_serve(&info, mount.volume_mode).err());
    if let Some(what) = lacked {
        return Err(MountsError::Unsupported(format!("the volume {} cannot be served: {what}", mount.volume_name)));
    }
    let request = VolumeServeRequest {
        name: mount.volume_name.clone(),
        mode: mount.volume_mode,
        overlay_disk: mount.overlay_disk.unwrap_or(0),
    };
    let serve = match volume_serve::execute(&handle, &request).await {
        Ok(serve) => serve,
        Err(error) => return Err(MountsError::Failed(format!("the volume {} could not be served: {error:?}", mount.volume_name))),
    };
    let capacity = daemon.live.capacity_cell(&mount.provider);
    if let Some(cell) = &capacity {
        cell.nudge();
    }
    Ok(Served {
        serve: Mutex::new(Some(Serve::Volume(Arc::new(serve)))),
        prefix: mount.volume_relative_path.clone(),
        capacity,
    })
}

/// One volume mount's serve, opened on the daemon the volume is on:
/// that daemon's `volumes fuse_serve`, as a connected tool's connect
/// is opened. The rules:
///
/// 1. **The record, and its one connection.** The record `id` names
///    is read; one that is gone is the failure. The daemon it reaches
///    is connected to as the account the record names, over the one
///    connection every use of that record shares.
/// 2. **Relayed along the route.** The serve is asked of that daemon
///    with the mount's `route` as the request's `relay`, so each
///    daemon along it serves the next hop as an account and judges
///    its `relay` there, and the last serves the volume itself,
///    judging that account's `fuse_serve` over it.
/// 3. **Nothing of that daemon's is checked here.** Not its
///    providers, not their capabilities, not the volume: a serve it
///    refuses is the failure, in its own answer's words.
/// 4. **No capacity is this daemon's.** The serve's layer is on the
///    volume's provider, which is no provider of this daemon's, so
///    there is none to nudge.
async fn serve_remote(daemon: &Arc<Daemon>, mount: &VolumeFuseMount, id: diverge_sdk::daemon::endpoints::daemons::DaemonId) -> Result<Served, MountsError> {
    let failed = |why: String| MountsError::Failed(format!("the volume {} on the daemon `{id}` could not be served: {why}", mount.volume_name));
    // Rule 1.
    let record = {
        let mut conn = daemon.store.acquire().await.map_err(|error| failed(error.to_string()))?;
        crate::store::daemons::by_id(&mut conn, id, false).await.map_err(|error| failed(error.to_string()))?
    };
    let Some(record) = record else {
        return Err(failed("no remote daemon's record has that ID".to_string()));
    };
    let peer = daemons::connect(daemon, &record).await.map_err(|error| failed(format!("{error:?}")))?;
    // Rule 2.
    let request = DaemonServeRequest {
        volume: reference::Volume {
            provider: mount.provider.clone(),
            name: mount.volume_name.clone(),
        },
        mode: mount.volume_mode,
        overlay_disk: mount.overlay_disk,
        relay: (!mount.route.is_empty()).then(|| Relay(mount.route.clone())),
    };
    // Rule 3.
    let serve = daemon_serve::execute(&peer.connected.handle, &request).await.map_err(|error| failed(error.to_string()))?;
    Ok(Served {
        serve: Mutex::new(Some(Serve::Volume(Arc::new(serve)))),
        prefix: mount.volume_relative_path.clone(),
        // Rule 4.
        capacity: None,
    })
}
