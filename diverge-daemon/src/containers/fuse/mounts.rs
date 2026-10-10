//! One run's mounts, built and asked.

use std::collections::HashMap;

use bytes::Bytes;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::server::channel_request::Frame as Ask;
use diverge_sdk::daemon::endpoints::agents::create::client::request::FuseMount as VolumeFuseMount;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::client::Listed;
use diverge_sdk::provider::endpoints::containers::serve::client::execute::{self as container_serve, ExecuteHandle as ContainerServe};
use diverge_sdk::provider::endpoints::containers::serve::client::request::Frame as ContainerServeRequest;
use diverge_sdk::provider::endpoints::volumes::serve::client::execute::{self as volume_serve, ExecuteHandle as VolumeServe};
use diverge_sdk::provider::endpoints::volumes::serve::client::request::Frame as VolumeServeRequest;
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::fuse::Attrs;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::containers::fuse::stat::Stat;
use diverge_sdk::shared::containers::request::FuseMount;
use diverge_sdk::shared::filetree::response::Frame;
use diverge_sdk::wire::client::channel::Channel;
use tokio::sync::{Mutex, broadcast};
use tokio::task::AbortHandle;

use super::{bridge, watch};
use crate::daemon::Daemon;

/// The scope a mount's bytes come from: a volume served by its
/// provider, or a subtree of a running container served by the
/// provider it runs on. The two answer the same nine asks in the same
/// frames, so a mount is asked without knowing which.
pub enum Serve {
    /// A `volumes::serve`.
    Volume(VolumeServe),
    /// A `containers::serve`: an agent's path, into its dependency.
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
            Serve::Volume(serve) => serve.stop().await,
            Serve::Container(serve) => serve.stop().await,
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
}

/// The mounts of one run: what the provider is told to mount, and
/// where each is answered from.
pub struct Mounts {
    sources: HashMap<String, Served>,
    /// The file mounts, as the run request names them.
    pub files: Vec<FuseMount>,
    /// The directory mounts, likewise.
    pub directories: Vec<FuseMount>,
    /// Every change made under a mount, as a frame from the
    /// container's root, for whoever watches the container.
    pub(super) changes: broadcast::Sender<Frame>,
    /// The watches of the served trees, for the mounts whose source
    /// streams its own changes: ended with the mounts.
    watchers: Mutex<Vec<AbortHandle>>,
}

impl Mounts {
    /// No mount: a connected tool's, whose container is somebody
    /// else's.
    pub fn empty() -> Mounts {
        Mounts {
            sources: HashMap::new(),
            files: Vec::new(),
            directories: Vec::new(),
            changes: broadcast::channel(256).0,
            watchers: Mutex::new(Vec::new()),
        }
    }

    /// The mounts for a record's run: its cross-provider mounts, every
    /// volume's serve opened on its provider. The first that cannot
    /// be is the failure, in a sentence, and what was opened is let
    /// go.
    pub async fn build(daemon: &Daemon, volume_files: &[VolumeFuseMount], volume_directories: &[VolumeFuseMount]) -> Result<Mounts, String> {
        let mut mounts = Mounts::empty();
        let mut v = 0usize;
        for (mount, directory) in volume_files
            .iter()
            .map(|mount| (mount, false))
            .chain(volume_directories.iter().map(|mount| (mount, true)))
        {
            v += 1;
            let id = format!("v{v}");
            let Some(handle) = daemon.live.provider(&mount.provider).await else {
                mounts.stop().await;
                return Err(format!("the provider of the volume {} is not connected now", mount.volume_name));
            };
            let request = VolumeServeRequest {
                name: mount.volume_name.clone(),
                mode: mount.volume_mode,
                overlay_disk: mount.overlay_disk.unwrap_or(0),
            };
            let serve = match volume_serve::execute(&handle, &request).await {
                Ok(serve) => serve,
                Err(error) => {
                    mounts.stop().await;
                    return Err(format!("the volume {} could not be served: {error:?}", mount.volume_name));
                }
            };
            mounts.sources.insert(
                id.clone(),
                Served {
                    serve: Mutex::new(Some(Serve::Volume(serve))),
                    prefix: mount.volume_relative_path.clone(),
                },
            );
            let fuse = FuseMount {
                container_path: mount.container_path.clone(),
                id,
            };
            if directory {
                mounts.directories.push(fuse);
            } else {
                mounts.files.push(fuse);
            }
        }
        Ok(mounts)
    }

    /// The mounts for a dependency's run: the agent's paths the
    /// template names, each served by the agent's provider from the
    /// agent's container — a directory whole, a file with the
    /// directory holding it — and watched for every change, the
    /// agent's own included. The first that cannot be served is the
    /// failure, in a sentence, and what was opened is let go.
    pub async fn build_dependency(daemon: &Daemon, agent_provider: &Identity, agent_container: &str, template: &Template) -> Result<Mounts, String> {
        let mut mounts = Mounts::empty();
        let Some(handle) = daemon.live.provider(agent_provider).await else {
            return Err("the agent's provider is not connected now".to_string());
        };
        let mut d = 0usize;
        for (mount, directory) in template
            .fuse_file_mounts
            .iter()
            .map(|mount| (mount, false))
            .chain(template.fuse_directory_mounts.iter().map(|mount| (mount, true)))
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
                    return Err("a file mount names the agent's root".to_string());
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
                    return Err(format!("the agent's path {} could not be served: {error:?}", mount.agent_path.join("/")));
                }
            };
            let tree = match serve.filetree().await {
                Ok(tree) => tree,
                Err(error) => {
                    serve.stop().await;
                    mounts.stop().await;
                    return Err(format!("the agent's path {} could not be watched: {error:?}", mount.agent_path.join("/")));
                }
            };
            let watcher = tokio::spawn(watch::relay(tree, mounts.changes.clone(), mount.tool_path.clone(), prefix.clone()));
            mounts.watchers.lock().await.push(watcher.abort_handle());
            mounts.sources.insert(
                id.clone(),
                Served {
                    serve: Mutex::new(Some(Serve::Container(serve))),
                    prefix,
                },
            );
            let fuse = FuseMount {
                container_path: mount.tool_path.clone(),
                id,
            };
            if directory {
                mounts.directories.push(fuse);
            } else {
                mounts.files.push(fuse);
            }
        }
        Ok(mounts)
    }

    /// The run is over: every watch ended, every serve stopped.
    pub async fn stop(&self) {
        for watcher in self.watchers.lock().await.drain(..) {
            watcher.abort();
        }
        for served in self.sources.values() {
            if let Some(serve) = served.serve.lock().await.take() {
                serve.stop().await;
            }
        }
    }

    /// The source the ask names, if the id is one of this run's.
    fn source(&self, id: &str) -> Result<&Served, String> {
        self.sources.get(id).ok_or_else(|| format!("no mount is served as {id}"))
    }

    /// Whether the mount's source streams its own changes, so that the
    /// daemon need not report the ones it makes.
    async fn streams(&self, id: &str) -> bool {
        match self.sources.get(id) {
            Some(served) => matches!(&*served.serve.lock().await, Some(Serve::Container(_))),
            None => false,
        }
    }

    /// What is at the path, if anything.
    pub async fn stat(&self, id: &str, path: &str) -> Result<Option<Stat>, String> {
        let Served { serve, prefix } = self.source(id)?;
        bridge::stat(serve, prefix, path).await
    }

    /// A piece of the file at the path.
    pub async fn read(&self, id: &str, path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
        let Served { serve, prefix } = self.source(id)?;
        bridge::read(serve, prefix, path, offset, length).await
    }

    /// A piece written into the file at the path.
    pub async fn write(&self, id: &str, path: &str, offset: u64, bytes: Bytes) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::write(serve, prefix, path, offset, &bytes).await?;
        if !self.streams(id).await {
            self.changed(id, path, false).await;
        }
        Ok(())
    }

    /// The file at the path made `size` long.
    pub async fn truncate(&self, id: &str, path: &str, size: u64) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::truncate(serve, prefix, path, size).await?;
        if !self.streams(id).await {
            self.changed(id, path, false).await;
        }
        Ok(())
    }

    /// The attributes of what is at the path changed.
    pub async fn setattr(&self, id: &str, path: &str, attrs: Attrs) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::setattr(serve, prefix, path, attrs).await
    }

    /// The entries of the directory at the path.
    pub async fn list(&self, id: &str, path: &str) -> Result<Option<Vec<Listed>>, String> {
        let Served { serve, prefix } = self.source(id)?;
        bridge::list(serve, prefix, path).await
    }

    /// What is at the path removed.
    pub async fn remove(&self, id: &str, path: &str) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::remove(serve, prefix, path).await?;
        if !self.streams(id).await {
            self.removed(id, path);
        }
        Ok(())
    }

    /// What is at `from` moved to `to`.
    pub async fn rename(&self, id: &str, from: &str, to: &str) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::rename(serve, prefix, from, to).await?;
        if !self.streams(id).await {
            self.removed(id, from);
            self.changed(id, to, true).await;
        }
        Ok(())
    }

    /// A directory made at the path.
    pub async fn mkdir(&self, id: &str, path: &str) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::mkdir(serve, prefix, path).await?;
        if !self.streams(id).await {
            self.changed(id, path, true).await;
        }
        Ok(())
    }
}
