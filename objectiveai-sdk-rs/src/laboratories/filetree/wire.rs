//! Wire types for the laboratory `/filetree` SSE endpoint.
//!
//! The endpoint yields a full recursive tree [`Snapshot`] as its first
//! event, then live [`Upserted`] / [`Removed`] deltas as the container
//! filesystem changes. The snapshot is a recursive [`FileTreeNode`];
//! the deltas are flat, path-keyed [`FileTreeEntry`]s, so a consumer
//! folds the snapshot into a `path → entry` map and applies deltas by
//! path.
//!
//! [`Snapshot`]: FileTreeEvent::Snapshot
//! [`Upserted`]: FileTreeEvent::Upserted
//! [`Removed`]: FileTreeEvent::Removed

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What a filesystem entry is. `Symlink` is reported for the link
/// itself (never followed), so a broken or looping link renders as a
/// leaf rather than confusing the tree.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "laboratories.filetree.FileKind")]
pub enum FileKind {
    /// A regular file.
    #[schemars(title = "File")]
    File,
    /// A directory.
    #[schemars(title = "Dir")]
    Dir,
    /// A symbolic link (the link itself, not its target).
    #[schemars(title = "Symlink")]
    Symlink,
}

/// One node of the recursive tree snapshot. The ROOT node's `name` is
/// the watched path itself; every other node's `name` is its basename,
/// and its full path is its position in the tree.
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
#[schemars(rename = "laboratories.filetree.FileTreeNode")]
pub struct FileTreeNode {
    /// Basename of this entry (the root node carries the watched path).
    pub name: String,
    /// What this entry is.
    pub kind: FileKind,
    /// Size in bytes. Files only — `None` for directories and symlinks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub size: Option<u64>,
    /// Creation time (unix seconds), when the filesystem records a
    /// birth time. `None` when unsupported — best-effort display
    /// metadata, never a load-bearing field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
    /// Last-modified time (unix seconds) — files AND directories (a
    /// directory's mtime tracks entry add/remove). `None` when the
    /// stat couldn't be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub modified_at: Option<i64>,
    /// The agent that created this entry, when known. Reserved for the
    /// attribution engine; currently always `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_by: Option<String>,
    /// The agent that last modified this entry, when known. Reserved
    /// for the attribution engine; currently always `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub modified_by: Option<String>,
    /// Child entries — directories only. `None` for files and
    /// symlinks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub children: Option<Vec<FileTreeNode>>,
}

impl FileTreeNode {
    /// Flattens the tree into `(path, entry)` pairs, one per node
    /// EXCEPT the root itself (the root is the watched dir; its
    /// children carry paths relative to it). Paths are `/`-separated
    /// and relative to the watched root.
    pub fn flatten(&self) -> Vec<(String, FileTreeEntry)> {
        let mut out = Vec::new();
        if let Some(children) = &self.children {
            for child in children {
                child.flatten_into(String::new(), &mut out);
            }
        }
        out
    }

    fn flatten_into(&self, prefix: String, out: &mut Vec<(String, FileTreeEntry)>) {
        let path = if prefix.is_empty() {
            self.name.clone()
        } else {
            format!("{prefix}/{}", self.name)
        };
        out.push((
            path.clone(),
            FileTreeEntry {
                path: path.clone(),
                kind: self.kind,
                size: self.size,
                created_at: self.created_at,
                modified_at: self.modified_at,
                created_by: self.created_by.clone(),
                modified_by: self.modified_by.clone(),
            },
        ));
        if let Some(children) = &self.children {
            for child in children {
                child.flatten_into(path.clone(), out);
            }
        }
    }
}

/// One flat filesystem entry — the shape of a live delta. Same scalar
/// metadata as [`FileTreeNode`], plus the entry's `path` (relative to
/// the watched root, `/`-separated) and no `children`.
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
#[schemars(rename = "laboratories.filetree.FileTreeEntry")]
pub struct FileTreeEntry {
    /// Path relative to the watched root, `/`-separated.
    pub path: String,
    /// What this entry is.
    pub kind: FileKind,
    /// Size in bytes. Files only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub size: Option<u64>,
    /// Creation time (unix seconds), when the filesystem records it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
    /// Last-modified time (unix seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub modified_at: Option<i64>,
    /// The agent that created this entry, when known. Reserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_by: Option<String>,
    /// The agent that last modified this entry, when known. Reserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub modified_by: Option<String>,
}

/// One event on the `/filetree` SSE stream. The first is always a
/// [`Snapshot`](FileTreeEvent::Snapshot); every later one upserts or
/// removes a single path as the filesystem changes.
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
    /// The full tree, sent once immediately on connect.
    #[schemars(title = "Snapshot")]
    Snapshot {
        /// The watched root and its recursive contents.
        root: FileTreeNode,
    },
    /// A single path came into existence or changed.
    #[schemars(title = "Upserted")]
    Upserted {
        /// The new/updated entry.
        entry: FileTreeEntry,
    },
    /// A single path (and, if a directory, its whole subtree) was
    /// removed.
    #[schemars(title = "Removed")]
    Removed {
        /// Path relative to the watched root, `/`-separated.
        path: String,
    },
}
