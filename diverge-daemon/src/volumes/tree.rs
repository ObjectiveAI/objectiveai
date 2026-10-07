//! A volume's tree, walked at rest, and what is at a path in it.

use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::volumes::filetree::client::{execute, request};
use diverge_sdk::shared::filetree::response::Node;

use super::{Fail, provider};
use crate::daemon::Daemon;

/// What is at a path.
#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    /// A file: its node.
    File(Node),
    /// A directory: its entries.
    Directory(Vec<Node>),
    /// Nothing, or a symlink, which is neither.
    Missing,
}

/// The tree of the directory at `path` in the volume, as the provider
/// walks it once at rest; the root for an empty path.
pub async fn tree(daemon: &Daemon, volume: &reference::Volume, path: &[String]) -> Result<Vec<Node>, Fail> {
    let handle = provider::handle(daemon, &volume.provider).await?;
    let request = request::Frame {
        name: volume.name.clone(),
        path: path.to_vec(),
    };
    match execute::execute(&handle, &request).await {
        Ok(nodes) => Ok(nodes),
        Err(execute::ExecuteError::Provider(error)) => Err(Fail::refusal(&error)),
        Err(error) => Err(Fail::failed(&error)),
    }
}

/// What is at `path` in the volume: the parent walked, and the entry
/// found in it; the root itself for an empty path.
pub async fn entry_at(daemon: &Daemon, volume: &reference::Volume, path: &[String]) -> Result<Found, Fail> {
    let Some((name, parent)) = path.split_last() else {
        return Ok(Found::Directory(tree(daemon, volume, &[]).await?));
    };
    let siblings = match tree(daemon, volume, parent).await {
        Ok(siblings) => siblings,
        Err(Fail::NotFound) => return Ok(Found::Missing),
        Err(error) => return Err(error),
    };
    Ok(found(&siblings, name))
}

/// The entry named among `nodes`, as what is at a path.
pub fn found(nodes: &[Node], name: &str) -> Found {
    match nodes.iter().find(|node| name_of(node) == name) {
        Some(Node::File { .. }) => Found::File(nodes.iter().find(|node| name_of(node) == name).cloned().unwrap_or(Node::File {
            name: name.to_string(),
            size: None,
            created_at: None,
            modified_at: None,
        })),
        Some(Node::Directory { children, .. }) => Found::Directory(children.clone()),
        Some(Node::Symlink { .. }) | None => Found::Missing,
    }
}

/// What is at `path` within a tree already walked: the root's entries
/// for an empty path, descending by name.
pub fn at<'a>(nodes: &'a [Node], path: &[String]) -> Option<&'a Node> {
    let (first, rest) = path.split_first()?;
    let node = nodes.iter().find(|node| name_of(node) == first)?;
    if rest.is_empty() {
        return Some(node);
    }
    match node {
        Node::Directory { children, .. } => at(children, rest),
        _ => None,
    }
}

/// Every file under `nodes`, by its path relative to them, in
/// bytewise order of the joined path; symlinks are not files.
pub fn files_under(nodes: &[Node]) -> Vec<Vec<String>> {
    let mut files = Vec::new();
    let mut prefix = Vec::new();
    walk(nodes, &mut prefix, &mut files);
    files.sort_by(|a, b| a.join("/").as_bytes().cmp(b.join("/").as_bytes()));
    files
}

/// The walk under `files_under`.
fn walk(nodes: &[Node], prefix: &mut Vec<String>, files: &mut Vec<Vec<String>>) {
    for node in nodes {
        match node {
            Node::File { name, .. } => {
                let mut path = prefix.clone();
                path.push(name.clone());
                files.push(path);
            }
            Node::Directory { name, children, .. } => {
                prefix.push(name.clone());
                walk(children, prefix, files);
                prefix.pop();
            }
            Node::Symlink { .. } => {}
        }
    }
}

/// A node's name, whichever it is.
pub fn name_of(node: &Node) -> &str {
    match node {
        Node::File { name, .. } | Node::Directory { name, .. } | Node::Symlink { name, .. } => name,
    }
}

/// Every directory in `nodes` with `changes` as given, through the
/// whole tree.
pub fn with_changes(nodes: &mut [Node], changes: bool) {
    for node in nodes {
        if let Node::Directory {
            changes: own, children, ..
        } = node
        {
            *own = changes;
            with_changes(children, changes);
        }
    }
}
