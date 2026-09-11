//! One notify event, as the frames it means.

use std::fs;
use std::path::{Component, Path};
use std::sync::Mutex;

use diverge_provider_sdk::shared::filetree::response::Frame;
use notify::EventKind;
use notify::event::{ModifyKind, RenameMode};

use super::{Ignore, Watch, node};

/// What an event came to.
pub enum Mapped {
    /// Zero or more frames, in order.
    Frames(Vec<Frame>),
    /// The watch lost events and the tree can no longer be trusted:
    /// the caller re-walks and sends a fresh snapshot.
    Resync,
}

/// Map one event to what the stream sends for it.
///
/// The rules, one per kind of event:
///
/// - **Rescan flagged** — the inotify queue overflowed — is
///   [`Resync`](Mapped::Resync), whatever else the event says.
/// - **Create** — a directory is registered for watching first,
///   which matters only where the tree's registration degraded (see
///   [`Watch::register`]) and decides its `changes`; then the node is
///   read. Present: `Inserted` with its complete value, a directory
///   with its subtree. Already gone: nothing; the removal that
///   follows will say so.
/// - **Rename, the leaving half** (`From`) — `Removed`. Whether the
///   node went elsewhere in the tree or out of it, this path no
///   longer holds it.
/// - **Rename, the arriving half** (`To`) — as a create.
/// - **Rename, both halves paired** (`Both`) — nothing: its halves
///   were already reported, one each. There is no move frame.
/// - **Any other modification** — data, metadata, a rename notify
///   could not tell apart — the node is read. Present: `Modified`
///   with its complete value. Gone: `Removed`.
/// - **Remove** — `Removed`.
/// - **Access**, and anything else — nothing; the tree did not change.
///
/// A path that is excluded, or is the root itself, is dropped; the
/// path a frame carries is the event's components after `/`. A
/// watched directory being renamed yields its `From` twice — once
/// from its parent's watch, once from its own — which is a second
/// `Removed` of a path already empty, and the fold drops it.
pub fn map(event: notify::Event, ignore: &Ignore, watch: &Mutex<Watch>) -> Mapped {
    if event.need_rescan() {
        return Mapped::Resync;
    }
    let arriving = matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(RenameMode::To))
    );
    let dark = {
        let Ok(mut watch) = watch.lock() else {
            return Mapped::Frames(Vec::new());
        };
        if arriving {
            for path in &event.paths {
                if !ignore.excluded(path)
                    && fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
                {
                    let _ = watch.register(path, ignore);
                }
            }
        }
        watch.dark()
    };
    let mut frames = Vec::new();
    match event.kind {
        EventKind::Create(_)
        | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            for path in &event.paths {
                let Some(components) = components(path, ignore) else {
                    continue;
                };
                if let Some(node) = node(path, ignore, &dark) {
                    frames.push(Frame::Inserted {
                        path: components,
                        node,
                    });
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From))
        | EventKind::Remove(_) => {
            for path in &event.paths {
                if let Some(components) = components(path, ignore) {
                    frames.push(Frame::Removed { path: components });
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {}
        EventKind::Modify(_) => {
            for path in &event.paths {
                let Some(components) = components(path, ignore) else {
                    continue;
                };
                frames.push(match node(path, ignore, &dark) {
                    Some(node) => Frame::Modified {
                        path: components,
                        node,
                    },
                    None => Frame::Removed { path: components },
                });
            }
        }
        EventKind::Access(_) | EventKind::Any | EventKind::Other => {}
    }
    Mapped::Frames(frames)
}

/// The path's components after `/`: `None` for the root itself and
/// for an excluded path.
fn components(path: &Path, ignore: &Ignore) -> Option<Vec<String>> {
    if ignore.excluded(path) {
        return None;
    }
    let components: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    if components.is_empty() {
        None
    } else {
        Some(components)
    }
}
