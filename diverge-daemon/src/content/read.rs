//! Reading a held resource: its files, and its tree.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use diverge_sdk::shared::filetree::response::Node;

use super::Error;

/// What is at a path inside a resource.
#[derive(Debug)]
pub enum Found {
    /// A file, at its path on disk.
    File(PathBuf),
    /// A directory, at its path on disk.
    Directory(PathBuf),
}

/// What is at `components` under `root`, if anything.
pub async fn at(root: &Path, components: &[String]) -> Option<Found> {
    let path = super::paths::join(root, components);
    let meta = tokio::fs::metadata(&path).await.ok()?;
    if meta.is_dir() {
        Some(Found::Directory(path))
    } else if meta.is_file() {
        Some(Found::File(path))
    } else {
        None
    }
}

/// Every file under `dir`, each as its path relative to `dir` and
/// its path on disk, in bytewise order of the relative paths — the
/// order a download sends them in. Walked on a blocking task.
pub async fn files(dir: &Path) -> Result<Vec<(Vec<String>, PathBuf)>, Error> {
    let dir = dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut found = Vec::new();
        walk_files(&dir, &mut Vec::new(), &mut found)?;
        found.sort_by(|a, b| a.0.join("/").as_bytes().cmp(b.0.join("/").as_bytes()));
        Ok(found)
    })
    .await
    .unwrap_or_else(|_| Err(Error::Path(String::new())))
}

/// The files under `dir`, recursively, with `prefix` the components
/// so far.
fn walk_files(dir: &Path, prefix: &mut Vec<String>, found: &mut Vec<(Vec<String>, PathBuf)>) -> Result<(), Error> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().map_err(|source| Error::Io {
            path: entry.path(),
            source,
        })?;
        prefix.push(name);
        if meta.is_dir() {
            walk_files(&entry.path(), prefix, found)?;
        } else if meta.is_file() {
            found.push((prefix.clone(), entry.path()));
        }
        prefix.pop();
    }
    Ok(())
}

/// The tree of `dir`: its entries, each with everything beneath it,
/// as the nodes of a snapshot, every directory with `changes` false
/// — a resource never changes. Walked on a blocking task.
pub async fn tree(dir: &Path) -> Result<Vec<Node>, Error> {
    let dir = dir.to_path_buf();
    tokio::task::spawn_blocking(move || nodes(&dir))
        .await
        .unwrap_or_else(|_| Err(Error::Path(String::new())))
}

/// The nodes of `dir`'s entries, sorted bytewise by name.
fn nodes(dir: &Path) -> Result<Vec<Node>, Error> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = std::fs::symlink_metadata(entry.path()).map_err(|source| Error::Io {
            path: entry.path(),
            source,
        })?;
        let created_at = meta.created().ok().and_then(seconds);
        let modified_at = meta.modified().ok().and_then(seconds);
        let node = if meta.is_dir() {
            Node::Directory {
                name,
                created_at,
                modified_at,
                changes: false,
                children: nodes(&entry.path())?,
            }
        } else if meta.is_symlink() {
            let target = std::fs::read_link(entry.path()).map_err(|source| Error::Io {
                path: entry.path(),
                source,
            })?;
            Node::Symlink {
                name,
                path: target
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy().into_owned())
                    .collect(),
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
        };
        found.push(node);
    }
    found.sort_by(|a, b| name_of(a).as_bytes().cmp(name_of(b).as_bytes()));
    Ok(found)
}

/// A node's name.
fn name_of(node: &Node) -> &str {
    match node {
        Node::File { name, .. } | Node::Directory { name, .. } | Node::Symlink { name, .. } => name,
    }
}

/// A time as whole seconds since the epoch.
fn seconds(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH).ok().map(|since| since.as_secs())
}
