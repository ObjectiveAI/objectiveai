//! Files into and out of a running container, the daemon's own
//! mounts included.
//!
//! The provider's tree of a container leaves out the FUSE mounts the
//! daemon serves; [`tree`] splices each in at its mount point, every
//! directory under one `changes` true, and [`entry_at`] finds a path
//! in that whole. A path under a mount is read and written from the
//! mount's own source; any other through the run's channels.

use bytes::Bytes;
use diverge_sdk::shared::filetree::response::{Frame, Node, Root};
use futures_util::{StreamExt as _, stream};

use super::Opened;
use super::fuse::Mounts;
use crate::content::Pieces;
use crate::volumes::{self, Found};

/// The container's tree, whole: the provider's snapshot with every
/// mount spliced in at its path.
pub async fn tree(opened: &Opened) -> Result<Vec<Node>, String> {
    let mut stream = opened.filetree().await?;
    let first = loop {
        match stream.next().await {
            Some(Ok(Frame::Snapshot { children })) => break children,
            Some(Ok(_)) => continue,
            Some(Err(error)) => return Err(error.to_string()),
            None => return Err("the container's tree was not sent".to_string()),
        }
    };
    let mut root = Root(first);
    for (path, node) in opened.mounts().subtrees().await {
        root.update(Frame::Inserted { path, node });
    }
    Ok(root.0)
}

/// What is at `path` in the container, the mounts included.
pub async fn entry_at(opened: &Opened, path: &[String]) -> Result<Found, String> {
    let nodes = tree(opened).await?;
    if path.is_empty() {
        return Ok(Found::Directory(nodes));
    }
    Ok(match volumes::at(&nodes, path) {
        Some(node @ Node::File { .. }) => Found::File(node.clone()),
        Some(Node::Directory { children, .. }) => Found::Directory(children.clone()),
        Some(Node::Symlink { .. }) | None => Found::Missing,
    })
}

/// The file at `path` in the container, in pieces: from the mount's
/// source when the path is under a mount, else read through the run.
pub async fn read(opened: &Opened, path: &[String]) -> Result<Pieces, String> {
    if let Some((id, within)) = opened.mounts().mount_at(path) {
        return Ok(read_mount(opened.mounts().clone(), id, within));
    }
    opened.read(path.to_vec()).await
}

/// The file at `path` in the container replaced whole by `content`:
/// through the mount's source when the path is under a mount, else
/// written through the run.
pub async fn write(opened: &Opened, path: &[String], mut content: Pieces) -> Result<(), String> {
    let Some((id, within)) = opened.mounts().mount_at(path) else {
        return opened.write(path.to_vec(), content).await;
    };
    let mounts = opened.mounts();
    mounts.truncate(&id, &within, 0).await.map_err(|refused| format!("{refused:?}"))?;
    let mut offset = 0u64;
    while let Some(piece) = content.next().await {
        let piece = piece?;
        mounts
            .write(&id, &within, offset, piece.clone())
            .await
            .map_err(|refused| format!("{refused:?}"))?;
        offset += piece.len() as u64;
    }
    Ok(())
}

/// A mount's file, read in pieces of a chunk.
fn read_mount(mounts: std::sync::Arc<Mounts>, id: String, within: String) -> Pieces {
    Box::pin(stream::unfold((mounts, id, within, 0u64, false), |(mounts, id, within, offset, done)| async move {
        if done {
            return None;
        }
        let length = u32::try_from(diverge_sdk::CHUNK_SIZE).unwrap_or(u32::MAX);
        match mounts.read(&id, &within, offset, length).await {
            Ok(Some(bytes)) if bytes.is_empty() => None,
            Ok(Some(bytes)) => {
                let next = offset + bytes.len() as u64;
                Some((Ok::<Bytes, String>(bytes), (mounts, id, within, next, false)))
            }
            Ok(None) => Some((Err("the file is not there".to_string()), (mounts, id, within, offset, true))),
            Err(error) => Some((Err(error), (mounts, id, within, offset, true))),
        }
    }))
}
