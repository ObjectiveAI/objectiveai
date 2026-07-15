//! Wire types for the laboratory `/filetree` SSE endpoint.
//!
//! The endpoint yields a full recursive tree [`Snapshot`] as its first
//! event, then live [`Upserted`] / [`Removed`] deltas as the container
//! filesystem changes. A [`FileTreeNode`] is one node of the tree — a
//! tagged union of `file` / `directory` / `symlink`, where only a
//! directory carries `children`. The snapshot is the watched root's
//! child list; deltas address a node by its component `path`.
//!
//! [`Snapshot`]: FileTreeEvent::Snapshot
//! [`Upserted`]: FileTreeEvent::Upserted
//! [`Removed`]: FileTreeEvent::Removed

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One node of the filesystem tree — discriminated by `type`. A
/// `directory` carries its `children` inline; `file` and `symlink` are
/// leaves. Symlinks are the link itself (never followed), so a broken
/// or looping link renders as a leaf rather than confusing the tree.
///
/// Common metadata on every variant: `name` (basename), `created_at`
/// (birth time when the filesystem records one), `modified_at`
/// (mtime), and the reserved `created_by`/`modified_by` (the
/// attribution engine fills these later; always `None` today).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "laboratories.filetree.FileTreeNode")]
pub enum FileTreeNode {
    /// A regular file.
    #[schemars(title = "File")]
    File {
        /// Basename of this file.
        name: String,
        /// Size in bytes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
        size: Option<u64>,
        /// Creation time (unix seconds), when the filesystem records a
        /// birth time. `None` when unsupported — best-effort display
        /// metadata, never load-bearing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_option_i64)]
        created_at: Option<i64>,
        /// Last-modified time (unix seconds). `None` when the stat
        /// couldn't be read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_option_i64)]
        modified_at: Option<i64>,
        /// The agent that created this entry, when known. Reserved for
        /// the attribution engine; currently always `None`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        created_by: Option<String>,
        /// The agent that last modified this entry, when known.
        /// Reserved; currently always `None`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        modified_by: Option<String>,
    },
    /// A directory — carries its child nodes.
    #[schemars(title = "Directory")]
    Directory {
        /// Basename of this directory (the watched root is represented
        /// by the snapshot's child list, not by a root node).
        name: String,
        /// Creation time (unix seconds), when the filesystem records a
        /// birth time.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_option_i64)]
        created_at: Option<i64>,
        /// Last-modified time (unix seconds) — a directory's mtime
        /// tracks entry add/remove.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_option_i64)]
        modified_at: Option<i64>,
        /// The agent that created this entry, when known. Reserved.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        created_by: Option<String>,
        /// The agent that last modified this entry, when known.
        /// Reserved.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        modified_by: Option<String>,
        /// This directory's entries. Empty for an empty directory —
        /// never absent.
        children: Vec<FileTreeNode>,
    },
    /// A symbolic link (the link itself, not its target — never
    /// followed).
    #[schemars(title = "Symlink")]
    Symlink {
        /// Basename of this link.
        name: String,
        /// The link's target path, exactly as stored in the link
        /// (possibly relative, possibly dangling — never resolved or
        /// followed). `None` only when the readlink itself failed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        target: Option<String>,
        /// Creation time (unix seconds), when the filesystem records a
        /// birth time.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_option_i64)]
        created_at: Option<i64>,
        /// Last-modified time (unix seconds).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_option_i64)]
        modified_at: Option<i64>,
        /// The agent that created this entry, when known. Reserved.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        created_by: Option<String>,
        /// The agent that last modified this entry, when known.
        /// Reserved.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        modified_by: Option<String>,
    },
}

impl FileTreeNode {
    /// This node's basename.
    pub fn name(&self) -> &str {
        match self {
            FileTreeNode::File { name, .. }
            | FileTreeNode::Directory { name, .. }
            | FileTreeNode::Symlink { name, .. } => name,
        }
    }

    /// Mutable access to a directory's children — `None` for files and
    /// symlinks.
    pub fn children_mut(&mut self) -> Option<&mut Vec<FileTreeNode>> {
        match self {
            FileTreeNode::Directory { children, .. } => Some(children),
            _ => None,
        }
    }
}

/// One event on the `/filetree` SSE stream. The first is always a
/// [`Snapshot`](FileTreeEvent::Snapshot); every later one upserts or
/// removes a single node as the filesystem changes.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "laboratories.filetree.FileTreeEvent")]
pub enum FileTreeEvent {
    /// The full tree — the watched root's child nodes — sent once
    /// immediately on connect.
    #[schemars(title = "Snapshot")]
    Snapshot {
        /// The watched root's entries (its own identity is implicit —
        /// the caller knows the watched path).
        children: Vec<FileTreeNode>,
    },
    /// A single node came into existence or changed. A directory
    /// carries its whole (re-walked) subtree.
    #[schemars(title = "Upserted")]
    Upserted {
        /// The FULL component path to the node, relative to the
        /// watched root (the last element equals `node`'s name).
        path: Vec<String>,
        /// The new/updated node.
        node: FileTreeNode,
    },
    /// A single node (and, if a directory, its whole subtree) was
    /// removed.
    #[schemars(title = "Removed")]
    Removed {
        /// The FULL component path to the vanished node, relative to
        /// the watched root.
        path: Vec<String>,
    },
}

impl FileTreeEvent {
    /// Fold this event into a live tree (the watched root's children).
    /// `Snapshot` replaces the whole child set; `Upserted` inserts or
    /// replaces one node at its path (a directory node carries its
    /// whole subtree); `Removed` drops the node at its path (and,
    /// being a subtree, everything under it). Idempotent — replaying an
    /// already-applied event leaves the tree unchanged, so at-least-once
    /// delivery is safe.
    ///
    /// This is THE fold, shared by every holder of a materialized tree:
    /// the SDK's `FileTree` client, the laboratory host's per-lab
    /// state, and the CLI daemon's per-host state.
    pub fn apply(self, root: &mut Vec<FileTreeNode>) {
        match self {
            FileTreeEvent::Snapshot { children } => {
                *root = children;
            }
            FileTreeEvent::Upserted { path, node } => {
                let Some((leaf, parents)) = path.split_last() else {
                    // Empty path can't address a node — ignore.
                    return;
                };
                let siblings = descend_mut(root, parents);
                match siblings.iter().position(|c| c.name() == leaf) {
                    Some(i) => siblings[i] = node,
                    None => siblings.push(node),
                }
            }
            FileTreeEvent::Removed { path } => {
                let Some((leaf, parents)) = path.split_last() else {
                    return;
                };
                let siblings = descend_mut(root, parents);
                siblings.retain(|c| c.name() != leaf);
            }
        }
    }
}

/// Walk `comps` from `children`, following each component into its
/// directory's child list; a missing middle segment is created as a
/// synthetic empty directory (defensive — the server sends parents
/// before children, but a dropped frame shouldn't wedge the fold).
/// Returns the child list at the end of the walk.
fn descend_mut<'a>(
    mut children: &'a mut Vec<FileTreeNode>,
    comps: &[String],
) -> &'a mut Vec<FileTreeNode> {
    for comp in comps {
        let idx = match children.iter().position(|c| c.name() == comp) {
            Some(i) => i,
            None => {
                children.push(FileTreeNode::Directory {
                    name: comp.clone(),
                    created_at: None,
                    modified_at: None,
                    created_by: None,
                    modified_by: None,
                    children: Vec::new(),
                });
                children.len() - 1
            }
        };
        // A non-directory sitting where a directory should be gets
        // replaced with an empty directory so the walk can continue.
        if children[idx].children_mut().is_none() {
            children[idx] = FileTreeNode::Directory {
                name: comp.clone(),
                created_at: None,
                modified_at: None,
                created_by: None,
                modified_by: None,
                children: Vec::new(),
            };
        }
        children = children[idx].children_mut().expect("just ensured directory");
    }
    children
}
