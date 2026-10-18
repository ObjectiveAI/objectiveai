//! The tree, read from disk.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use diverge_sdk::shared::filetree::response::Node;

use super::Ignore;

/// The entries of `dir`, recursively — what a `Snapshot` carries for
/// the root and a directory node carries for itself.
///
/// Synchronous on purpose: one thread walking with `std::fs` beats a
/// task per entry hopping through the blocking pool, and the caller
/// runs this under `spawn_blocking` so nothing async waits on it. A
/// directory that cannot be read is empty, an entry that cannot be
/// read is absent, and an excluded entry does not exist. Entries come
/// in the order the directory yields them. `dark` is the watch's
/// list of directories it could not watch (see
/// [`Watch::dark`](super::Watch::dark)): a directory there, or under
/// one there, is walked like any other and reported with `changes`
/// false.
pub fn children(dir: &Path, ignore: &Ignore, dark: &[PathBuf]) -> Vec<Node> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| node(&entry.path(), ignore, dark))
        .collect()
}

/// The node at `path`, complete — a directory with its whole subtree.
///
/// `None` when there is nothing to describe: the path is gone or
/// cannot be stat'ed, is excluded, is the root itself, or is a link
/// whose target cannot be read — the SDK's rule that such a link is
/// a failure rather than a link with no target, and here the failure
/// is the node's absence. Links are never followed: a symlink is the
/// link, and its metadata is the link's own.
pub fn node(path: &Path, ignore: &Ignore, dark: &[PathBuf]) -> Option<Node> {
    if ignore.excluded(path) {
        return None;
    }
    let meta = fs::symlink_metadata(path).ok()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    let created_at = seconds(meta.created().ok());
    let modified_at = seconds(meta.modified().ok());
    let kind = meta.file_type();
    Some(if kind.is_dir() {
        Node::Directory {
            name,
            created_at,
            modified_at,
            changes: !dark.iter().any(|dark| path.starts_with(dark)),
            children: children(path, ignore, dark),
        }
    } else if kind.is_symlink() {
        let target = fs::read_link(path).ok()?;
        Node::Symlink {
            name,
            path: target_components(path, &target),
            created_at,
            modified_at,
        }
    } else {
        Node::File {
            name,
            size: Some(meta.len()),
            created_at,
            modified_at,
        }
    })
}

/// A link's target as components from the root, resolved LEXICALLY
/// against the link's own directory: an absolute target is its own
/// components; a relative one is walked from the link's parent, `.`
/// dropped and `..` stepping up, never above the root. The target is
/// never touched, so a dangling link resolves like any other.
fn target_components(link: &Path, target: &Path) -> Vec<String> {
    let mut components = Vec::new();
    if !target.is_absolute() {
        if let Some(parent) = link.parent() {
            push(&mut components, parent);
        }
    }
    push(&mut components, target);
    components
}

/// Walk `path`'s components onto `components`, lexically.
fn push(components: &mut Vec<String>, path: &Path) {
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                components.push(name.to_string_lossy().into_owned());
            }
            Component::ParentDir => {
                components.pop();
            }
            Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        }
    }
}

/// Unix seconds, or nothing where the filesystem records nothing.
fn seconds(time: Option<SystemTime>) -> Option<u64> {
    time?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}
