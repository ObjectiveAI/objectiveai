//! The mounts as nodes of the container's tree, and which mount a
//! path is under.

use diverge_sdk::provider::client::Listed;
use diverge_sdk::shared::containers::fuse::Kind;
use diverge_sdk::shared::filetree::response::Node;

use super::Mounts;

impl Mounts {
    /// The mount a path is under, if one: the mount's id and the path
    /// within it, `/`-joined as a FUSE ask names it — empty for a file
    /// mount, whose path is the file itself.
    pub fn mount_at(&self, path: &[String]) -> Option<(String, String)> {
        for mount in self.files.iter().chain(&self.directories) {
            let at = &mount.container_path;
            if path.len() >= at.len() && path[..at.len()] == at[..] {
                let within = path[at.len()..].join("/");
                return Some((mount.id.clone(), within));
            }
        }
        None
    }

    /// Every mount as the node to splice in at its container path:
    /// a file mount as its file, a directory mount as its tree, every
    /// directory `changes` true since the daemon reports every change
    /// it makes.
    pub async fn subtrees(&self) -> Vec<(Vec<String>, Node)> {
        let mut spliced = Vec::new();
        for mount in self.files.iter().chain(&self.directories) {
            let Some(name) = mount.container_path.last() else {
                continue;
            };
            if let Some(node) = self.node(&mount.id, "", name).await {
                spliced.push((mount.container_path.clone(), node));
            }
        }
        spliced
    }

    /// The node for what is at `path` within the mount, named `name`,
    /// walked whole.
    async fn node(&self, id: &str, path: &str, name: &str) -> Option<Node> {
        let stat = self.stat(id, path).await.ok()??;
        let created_at = Some(stat.ctime.secs);
        let modified_at = Some(stat.mtime.secs);
        match stat.kind {
            Kind::File => Some(Node::File {
                name: name.to_string(),
                size: Some(stat.size),
                created_at,
                modified_at,
            }),
            Kind::Directory => {
                let entries: Vec<Listed> = self.list(id, path).await.ok()??;
                let mut children = Vec::with_capacity(entries.len());
                for entry in entries {
                    let child_path = if path.is_empty() { entry.name.clone() } else { format!("{path}/{}", entry.name) };
                    if let Some(child) = Box::pin(self.node(id, &child_path, &entry.name)).await {
                        children.push(child);
                    }
                }
                Some(Node::Directory {
                    name: name.to_string(),
                    created_at,
                    modified_at,
                    changes: true,
                    children,
                })
            }
        }
    }

    /// The node for what is at `path` within the mount, for a change
    /// report: by the path's last component.
    pub async fn node_at(&self, id: &str, path: &str) -> Option<Node> {
        let name = path.rsplit('/').next().unwrap_or(path);
        let name = if name.is_empty() { self.mount_name(id)? } else { name.to_string() };
        self.node(id, path, &name).await
    }

    /// The last component of the mount's container path.
    fn mount_name(&self, id: &str) -> Option<String> {
        self.files
            .iter()
            .chain(&self.directories)
            .find(|mount| mount.id == id)
            .and_then(|mount| mount.container_path.last().cloned())
    }

    /// The container path of what is at `path` within the mount.
    pub fn container_path(&self, id: &str, path: &str) -> Option<Vec<String>> {
        let mount = self.files.iter().chain(&self.directories).find(|mount| mount.id == id)?;
        let mut full = mount.container_path.clone();
        full.extend(path.split('/').filter(|component| !component.is_empty()).map(str::to_string));
        Some(full)
    }
}
