//! What changes under a mount, told to whoever watches the container.

use diverge_sdk::shared::filetree::response::Frame;
use tokio::sync::broadcast;

use super::Mounts;

impl Mounts {
    /// Where a watch of the container hears every change under a
    /// mount, each frame's path from the container's root.
    pub fn subscribe(&self) -> broadcast::Receiver<Frame> {
        self.changes.subscribe()
    }

    /// What is at `path` within the mount changed, or appeared: told
    /// with its node as it is now.
    pub async fn changed(&self, id: &str, path: &str, inserted: bool) {
        let Some(full) = self.container_path(id, path) else {
            return;
        };
        let Some(node) = self.node_at(id, path).await else {
            return;
        };
        let frame = if inserted { Frame::Inserted { path: full, node } } else { Frame::Modified { path: full, node } };
        let _ = self.changes.send(frame);
    }

    /// What was at `path` within the mount is gone.
    pub fn removed(&self, id: &str, path: &str) {
        if let Some(full) = self.container_path(id, path) {
            let _ = self.changes.send(Frame::Removed { path: full });
        }
    }
}
