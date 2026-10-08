//! One run's mounts, built and asked.

use std::collections::HashMap;

use bytes::Bytes;
use diverge_sdk::daemon::endpoints::agents::create::client::request::FuseMount as VolumeFuseMount;
use diverge_sdk::provider::client::Listed;
use diverge_sdk::provider::endpoints::volumes::serve::client::execute::{self, ExecuteHandle as ServeHandle};
use diverge_sdk::provider::endpoints::volumes::serve::client::request::Frame as ServeRequest;
use diverge_sdk::shared::containers::fuse::Attrs;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::containers::fuse::stat::Stat;
use diverge_sdk::shared::containers::request::FuseMount;
use diverge_sdk::shared::filetree::response::Frame;
use tokio::sync::{Mutex, broadcast};

use super::bridge;
use crate::containers::Key;
use crate::daemon::Daemon;

/// Where one mount's bytes are: a volume of some provider, served to
/// the daemon; every ask is forwarded under the mount's path within
/// it.
pub struct Served {
    /// The serve scope, until it is stopped.
    pub serve: Mutex<Option<ServeHandle>>,
    /// The path within the volume the mount is at.
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
        }
    }

    /// The mounts for `key`'s run: the record's cross-provider
    /// mounts, every volume's serve opened on its provider. The first
    /// that cannot be is the failure, in a sentence, and what was
    /// opened is let go.
    pub async fn build(daemon: &Daemon, key: Key, volume_files: &[VolumeFuseMount], volume_directories: &[VolumeFuseMount]) -> Result<Mounts, String> {
        let _ = key;
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
            let request = ServeRequest {
                name: mount.volume_name.clone(),
                mode: mount.volume_mode,
                overlay_disk: mount.overlay_disk.unwrap_or(0),
            };
            let serve = match execute::execute(&handle, &request).await {
                Ok(serve) => serve,
                Err(error) => {
                    mounts.stop().await;
                    return Err(format!("the volume {} could not be served: {error:?}", mount.volume_name));
                }
            };
            mounts.sources.insert(
                id.clone(),
                Served {
                    serve: Mutex::new(Some(serve)),
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

    /// The run is over: every serve stopped.
    pub async fn stop(&self) {
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
        self.changed(id, path, false).await;
        Ok(())
    }

    /// The file at the path made `size` long.
    pub async fn truncate(&self, id: &str, path: &str, size: u64) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::truncate(serve, prefix, path, size).await?;
        self.changed(id, path, false).await;
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
        self.removed(id, path);
        Ok(())
    }

    /// What is at `from` moved to `to`.
    pub async fn rename(&self, id: &str, from: &str, to: &str) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::rename(serve, prefix, from, to).await?;
        self.removed(id, from);
        self.changed(id, to, true).await;
        Ok(())
    }

    /// A directory made at the path.
    pub async fn mkdir(&self, id: &str, path: &str) -> Result<(), Refused> {
        let Served { serve, prefix } = self.source(id).map_err(Refused::Error)?;
        bridge::mkdir(serve, prefix, path).await?;
        self.changed(id, path, true).await;
        Ok(())
    }
}
