//! The copy itself.

use std::sync::Arc;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::resources::Kind;
use diverge_sdk::daemon::reference;
use diverge_sdk::daemon::transfer::Destination;

use super::{Fail, Sink, Source, keep};
use crate::containers::files;
use crate::content;
use crate::daemon::Daemon;
use crate::volumes::{self, Found};

/// Copy what is at `path` in `source` to `destination`: the
/// destination resolved, every volume at either end taken, the
/// files found and landed one by one, everything let go after. The
/// new resource's id when the destination was one.
pub async fn copy(daemon: &Arc<Daemon>, source: Source, path: &[String], destination: Destination, creator: Creator) -> Result<Option<String>, Fail> {
    let sink = Sink::resolve(daemon, destination).await?;
    let mut taken: Vec<reference::Volume> = Vec::new();
    for volume in source.volume().into_iter().chain(sink.volume()) {
        if taken.contains(volume) {
            continue;
        }
        if !daemon.live.take_volume(volume).await {
            for held in &taken {
                daemon.live.release_volume(held).await;
            }
            sink.close(daemon).await;
            return Err(Fail::Held);
        }
        taken.push(volume.clone());
    }
    let outcome = land(daemon, &source, path, &sink, creator).await;
    for held in &taken {
        daemon.live.release_volume(held).await;
    }
    sink.close(daemon).await;
    outcome
}

/// The files found at the source and landed in the sink.
async fn land(daemon: &Arc<Daemon>, source: &Source, path: &[String], sink: &Sink, creator: Creator) -> Result<Option<String>, Fail> {
    let (kind, files): (Kind, Vec<Vec<String>>) = match source.entry_at(daemon, path).await? {
        Found::File(_) => (Kind::File, vec![Vec::new()]),
        Found::Directory(nodes) => (Kind::Directory, volumes::files_under(&nodes)),
        Found::Missing => return Err(Fail::NotFound),
    };
    match sink {
        Sink::Resource { description } => {
            let mut streams = Vec::with_capacity(files.len());
            for relative in files {
                let full = joined(path, &relative);
                streams.push((relative, source.read(daemon, &full).await?));
            }
            let received = content::ingest(&daemon.incoming(), kind, streams)
                .await
                .map_err(|error| Fail::Error(error.to_string()))?;
            keep(daemon, received, description.clone(), creator).await.map(Some).map_err(Fail::Error)
        }
        Sink::Volume { volume, path: at } => {
            for relative in files {
                let full = joined(path, &relative);
                let pieces = source.read(daemon, &full).await?;
                volumes::write(daemon, volume, &joined(at, &relative), pieces).await?;
            }
            Ok(None)
        }
        Sink::Opened { opened, path: at } => {
            for relative in files {
                let full = joined(path, &relative);
                let destination = joined(at, &relative);
                if let Source::Opened(from) = source
                    && from.provider() == opened.provider()
                    && let Some(id) = opened.container()
                    && from.mounts().mount_at(&full).is_none()
                    && opened.mounts().mount_at(&destination).is_none()
                {
                    from.touch();
                    opened.touch();
                    from.transfer(full, id.to_string(), destination).await.map_err(Fail::Error)?;
                    continue;
                }
                let pieces = source.read(daemon, &full).await?;
                opened.touch();
                files::write(opened, &destination, pieces).await.map_err(Fail::Error)?;
            }
            Ok(None)
        }
    }
}

/// `base` then `relative`.
fn joined(base: &[String], relative: &[String]) -> Vec<String> {
    let mut full = base.to_vec();
    full.extend_from_slice(relative);
    full
}
