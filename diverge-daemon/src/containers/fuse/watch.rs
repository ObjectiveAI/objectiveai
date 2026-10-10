//! A served tree's own changes, rerooted at the mount.

use diverge_sdk::provider::endpoints::volumes::serve::server::channel_response::filetree;
use diverge_sdk::shared::filetree::response::{Frame, Node};
use diverge_sdk::wire::client::channel::Channel;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::server::ServerFrame;
use tokio::sync::broadcast;

/// Relay the serve's filetree channel — the served tree as its
/// container holds it, then every change in it — into the mounts'
/// changes, each frame rerooted at the mount's container path: what a
/// watch of the container sees under the mount is exactly what the
/// served container sees at the served path. Ends with the channel:
/// the serve's finish, its error, or the connection gone.
pub async fn relay(mut tree: Channel, changes: broadcast::Sender<Frame>, container_path: Vec<String>, prefix: Vec<String>) {
    while let Some(bytes) = tree.response_receiver.recv().await {
        let payload = match ServerFrame::decode(&bytes) {
            Ok(ServerFrame::ChannelResponse { payload, .. }) => payload,
            Ok(ServerFrame::ChannelResponseFinish { .. }) => break,
            _ => continue,
        };
        let frame = match filetree::Frame::decode(payload) {
            Ok(filetree::Frame::Filetree(frame)) => frame,
            Ok(filetree::Frame::Error(_)) | Err(_) => break,
        };
        if let Some(rerooted) = reroot(frame, &container_path, &prefix) {
            let _ = changes.send(rerooted);
        }
    }
}

/// The served tree's frame as the container's: a snapshot is the
/// mount's node whole, inserted at the mount's path; a change at a
/// path under the prefix is the same change at the mount's path plus
/// the rest; a change elsewhere in the served tree is nothing to the
/// mount. The node at the prefix itself takes the mount's own name.
fn reroot(frame: Frame, container_path: &[String], prefix: &[String]) -> Option<Frame> {
    let name = container_path.last()?.clone();
    match frame {
        Frame::Snapshot { children } => Some(if prefix.is_empty() {
            Frame::Inserted {
                path: container_path.to_vec(),
                node: Node::Directory {
                    name,
                    created_at: None,
                    modified_at: None,
                    changes: true,
                    children,
                },
            }
        } else {
            match at(&children, prefix) {
                Some(node) => Frame::Inserted {
                    path: container_path.to_vec(),
                    node: renamed(node.clone(), name),
                },
                None => Frame::Removed {
                    path: container_path.to_vec(),
                },
            }
        }),
        Frame::Inserted { path, node } => {
            let rest = under(&path, prefix)?;
            let node = if rest.is_empty() { renamed(node, name) } else { node };
            Some(Frame::Inserted {
                path: joined(container_path, rest),
                node,
            })
        }
        Frame::Modified { path, node } => {
            let rest = under(&path, prefix)?;
            let node = if rest.is_empty() { renamed(node, name) } else { node };
            Some(Frame::Modified {
                path: joined(container_path, rest),
                node,
            })
        }
        Frame::Removed { path } => {
            let rest = under(&path, prefix)?;
            Some(Frame::Removed {
                path: joined(container_path, rest),
            })
        }
    }
}

/// The path's components past the prefix, or `None` for a path not
/// under it.
fn under<'a>(path: &'a [String], prefix: &[String]) -> Option<&'a [String]> {
    path.strip_prefix(prefix)
}

/// The mount's container path, then the rest.
fn joined(container_path: &[String], rest: &[String]) -> Vec<String> {
    let mut path = container_path.to_vec();
    path.extend(rest.iter().cloned());
    path
}

/// The node at `path` among `children`, walking directories.
fn at<'a>(children: &'a [Node], path: &[String]) -> Option<&'a Node> {
    let (first, rest) = path.split_first()?;
    let node = children.iter().find(|node| node_name(node) == first)?;
    if rest.is_empty() {
        return Some(node);
    }
    match node {
        Node::Directory { children, .. } => at(children, rest),
        _ => None,
    }
}

/// The node's name.
fn node_name(node: &Node) -> &str {
    match node {
        Node::File { name, .. } | Node::Directory { name, .. } | Node::Symlink { name, .. } => name,
    }
}

/// The node under another name: the mount's, where the served path's
/// last component differs from where it is mounted.
fn renamed(node: Node, name: String) -> Node {
    match node {
        Node::File {
            size,
            created_at,
            modified_at,
            ..
        } => Node::File {
            name,
            size,
            created_at,
            modified_at,
        },
        Node::Directory {
            created_at,
            modified_at,
            changes,
            children,
            ..
        } => Node::Directory {
            name,
            created_at,
            modified_at,
            changes,
            children,
        },
        Node::Symlink {
            path,
            created_at,
            modified_at,
            ..
        } => Node::Symlink {
            name,
            path,
            created_at,
            modified_at,
        },
    }
}
