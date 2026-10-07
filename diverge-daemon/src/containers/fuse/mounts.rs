//! One run's mounts, built and asked.

use std::collections::HashMap;
use std::path::PathBuf;

use bytes::Bytes;
use diverge_sdk::daemon::endpoints::agents::create::client::request::FuseMount as VolumeFuseMount;
use diverge_sdk::daemon::template::{ResourceDirectoryMount, ResourceFileMount, ResourceMode};
use diverge_sdk::provider::client::Listed;
use diverge_sdk::provider::endpoints::volumes::serve::client::execute::{self, ExecuteHandle as ServeHandle};
use diverge_sdk::provider::endpoints::volumes::serve::client::request::Frame as ServeRequest;
use diverge_sdk::shared::containers::fuse::Attrs;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::containers::fuse::stat::Stat;
use diverge_sdk::shared::containers::request::FuseMount;
use diverge_sdk::shared::filetree::response::Frame;
use tokio::sync::{Mutex, broadcast};

use super::{bridge, resources};
use crate::containers::Key;
use crate::content;
use crate::daemon::Daemon;

/// Where one mount's bytes are.
pub enum Source {
    /// One file of the daemon's content, or a copy of one.
    File {
        /// The file.
        path: PathBuf,
        /// Whether mutations are refused.
        read_only: bool,
    },
    /// A directory of the daemon's content, or a copy of one.
    Directory {
        /// The root.
        root: PathBuf,
        /// Whether mutations are refused.
        read_only: bool,
    },
    /// A volume of some provider, served to the daemon; every ask is
    /// forwarded under the mount's path within it.
    Volume {
        /// The serve scope, until it is stopped.
        serve: Mutex<Option<ServeHandle>>,
        /// The path within the volume the mount is at.
        prefix: Vec<String>,
    },
}

/// The mounts of one run: what the provider is told to mount, and
/// where each is answered from.
pub struct Mounts {
    sources: HashMap<String, Source>,
    /// The file mounts, as the run request names them.
    pub files: Vec<FuseMount>,
    /// The directory mounts, likewise.
    pub directories: Vec<FuseMount>,
    overlay: Option<PathBuf>,
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
            overlay: None,
            changes: broadcast::channel(256).0,
        }
    }

    /// The mounts for `key`'s run: the template's resource mounts and
    /// the record's cross-provider mounts, every source made ready —
    /// a resource found and copied when ephemeral, a volume's serve
    /// opened on its provider. The first that cannot be is the
    /// failure, in a sentence, and what was opened is let go.
    pub async fn build(
        daemon: &Daemon,
        key: Key,
        resource_files: &[ResourceFileMount],
        resource_directories: &[ResourceDirectoryMount],
        volume_files: &[VolumeFuseMount],
        volume_directories: &[VolumeFuseMount],
    ) -> Result<Mounts, String> {
        let mut mounts = Mounts::empty();
        let overlay = daemon.overlays.join(match key {
            Key::Agent(id) => format!("agent-{}", id.0),
            Key::Tool(id) => format!("tool-{}", id.0),
        });
        let mut n = 0usize;
        for mount in resource_files {
            n += 1;
            let id = format!("r{n}");
            let held = content::held(&daemon.resources, &mount.resource);
            if !tokio::fs::metadata(&held).await.map(|meta| meta.is_file()).unwrap_or(false) {
                mounts.stop().await;
                return Err(format!("the file resource {} is not held", mount.resource));
            }
            let (path, read_only) = match &mount.mode {
                ResourceMode::ReadOnly => (held, true),
                ResourceMode::Ephemeral { .. } => {
                    let copy = overlay.join(&id);
                    if let Err(error) = resources::copy(&held, &copy).await {
                        mounts.stop().await;
                        return Err(error);
                    }
                    mounts.overlay = Some(overlay.clone());
                    (copy, false)
                }
            };
            mounts.sources.insert(id.clone(), Source::File { path, read_only });
            mounts.files.push(FuseMount {
                container_path: mount.container_path.clone(),
                id,
            });
        }
        for mount in resource_directories {
            n += 1;
            let id = format!("r{n}");
            let held = content::join(&content::held(&daemon.resources, &mount.resource), &mount.resource_relative_path);
            if !tokio::fs::metadata(&held).await.map(|meta| meta.is_dir()).unwrap_or(false) {
                mounts.stop().await;
                return Err(format!("the directory resource {} has no directory at the path named", mount.resource));
            }
            let (root, read_only) = match &mount.mode {
                ResourceMode::ReadOnly => (held, true),
                ResourceMode::Ephemeral { .. } => {
                    let copy = overlay.join(&id);
                    if let Err(error) = resources::copy(&held, &copy).await {
                        mounts.stop().await;
                        return Err(error);
                    }
                    mounts.overlay = Some(overlay.clone());
                    (copy, false)
                }
            };
            mounts.sources.insert(id.clone(), Source::Directory { root, read_only });
            mounts.directories.push(FuseMount {
                container_path: mount.container_path.clone(),
                id,
            });
        }
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
                Source::Volume {
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

    /// The run is over: every serve stopped, the overlay removed.
    pub async fn stop(&self) {
        for source in self.sources.values() {
            if let Source::Volume { serve, .. } = source
                && let Some(serve) = serve.lock().await.take()
            {
                serve.stop().await;
            }
        }
        if let Some(overlay) = &self.overlay {
            let _ = tokio::fs::remove_dir_all(overlay).await;
        }
    }

    /// The source the ask names, if the id is one of this run's.
    fn source(&self, id: &str) -> Result<&Source, String> {
        self.sources.get(id).ok_or_else(|| format!("no mount is served as {id}"))
    }

    /// What is at the path, if anything.
    pub async fn stat(&self, id: &str, path: &str) -> Result<Option<Stat>, String> {
        match self.source(id)? {
            Source::Volume { serve, prefix } => bridge::stat(serve, prefix, path).await,
            local => resources::stat(local, path).await,
        }
    }

    /// A piece of the file at the path.
    pub async fn read(&self, id: &str, path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
        match self.source(id)? {
            Source::Volume { serve, prefix } => bridge::read(serve, prefix, path, offset, length).await,
            local => resources::read(local, path, offset, length).await,
        }
    }

    /// A piece written into the file at the path.
    pub async fn write(&self, id: &str, path: &str, offset: u64, bytes: Bytes) -> Result<(), Refused> {
        match self.source(id).map_err(Refused::Error)? {
            Source::Volume { serve, prefix } => bridge::write(serve, prefix, path, offset, &bytes).await,
            local => resources::write(local, path, offset, &bytes).await,
        }?;
        self.changed(id, path, false).await;
        Ok(())
    }

    /// The file at the path made `size` long.
    pub async fn truncate(&self, id: &str, path: &str, size: u64) -> Result<(), Refused> {
        match self.source(id).map_err(Refused::Error)? {
            Source::Volume { serve, prefix } => bridge::truncate(serve, prefix, path, size).await,
            local => resources::truncate(local, path, size).await,
        }?;
        self.changed(id, path, false).await;
        Ok(())
    }

    /// The attributes of what is at the path changed.
    pub async fn setattr(&self, id: &str, path: &str, attrs: Attrs) -> Result<(), Refused> {
        match self.source(id).map_err(Refused::Error)? {
            Source::Volume { serve, prefix } => bridge::setattr(serve, prefix, path, attrs).await,
            local => resources::setattr(local, path).await,
        }
    }

    /// The entries of the directory at the path.
    pub async fn list(&self, id: &str, path: &str) -> Result<Option<Vec<Listed>>, String> {
        match self.source(id)? {
            Source::Volume { serve, prefix } => bridge::list(serve, prefix, path).await,
            local => resources::list(local, path).await,
        }
    }

    /// What is at the path removed.
    pub async fn remove(&self, id: &str, path: &str) -> Result<(), Refused> {
        match self.source(id).map_err(Refused::Error)? {
            Source::Volume { serve, prefix } => bridge::remove(serve, prefix, path).await,
            local => resources::remove(local, path).await,
        }?;
        self.removed(id, path);
        Ok(())
    }

    /// What is at `from` moved to `to`.
    pub async fn rename(&self, id: &str, from: &str, to: &str) -> Result<(), Refused> {
        match self.source(id).map_err(Refused::Error)? {
            Source::Volume { serve, prefix } => bridge::rename(serve, prefix, from, to).await,
            local => resources::rename(local, from, to).await,
        }?;
        self.removed(id, from);
        self.changed(id, to, true).await;
        Ok(())
    }

    /// A directory made at the path.
    pub async fn mkdir(&self, id: &str, path: &str) -> Result<(), Refused> {
        match self.source(id).map_err(Refused::Error)? {
            Source::Volume { serve, prefix } => bridge::mkdir(serve, prefix, path).await,
            local => resources::mkdir(local, path).await,
        }?;
        self.changed(id, path, true).await;
        Ok(())
    }
}
