//! Host-side mount watching: the native piece of the unified per-lab
//! filetree.
//!
//! Mounted host folders are invisible to the in-container `/filetree`
//! stream (the mount protocol delivers no change events and walks far
//! slower than native, so the MCP is told to ignore them). This
//! module watches each mounted host path NATIVELY — inotify on Linux,
//! FSEvents on macOS, ReadDirectoryChangesW on Windows, all via the
//! `notify` crate — and grafts the mount's tree and live deltas into
//! the per-lab filetree the host materializes and pushes to daemons.
//! Downstream (daemon, viewer) sees ONE tree per laboratory and never
//! learns mounts exist.
//!
//! ## One watcher per host path
//!
//! Watches are keyed by the CANONICALIZED host path: any number of
//! laboratories mounting the same directory (at any container paths)
//! share one recursive watcher, one materialized mount-space tree,
//! and one pump task. A watch starts when the first mounting
//! laboratory's container starts and stops when the last one stops —
//! watcher count always returns to zero with the containers (no
//! leak). Single-FILE mounts are supported: the watcher covers the
//! file's PARENT directory non-recursively (survives editor
//! atomic-save renames on every platform) with events filtered to the
//! one path.
//!
//! ## Lock discipline (load-bearing)
//!
//! [`crate::host::HostServer`]'s `attach_lock` is the outermost lock
//! and the linearization point for all filetree emission.
//! [`MountWatch::subscribers`] and [`MountWatch::tree`] are SYNC leaf
//! locks never held across an await — all async work (walking,
//! stat'ing) happens before locking; lock holds are fold/clone only.
//! The pump folds `tree` BEFORE emitting, and every graft reads
//! `tree` and emits under ONE `attach_lock` hold — so any graft
//! ordered after a delta already contains that delta. Container
//! `Snapshot` folds wipe grafted subtrees; `HostServer::filetree_event`
//! re-grafts from [`MountRegistry::grafts_for`] in the same lock hold,
//! which makes every ordering (walk-before-snapshot, snapshot-before-
//! walk, pump reconnects) correct.
//!
//! ## Known punts
//!
//! - Nested mounts from DIFFERENT host dirs (`/w/a` + `/w/a/b`): the
//!   outer watcher's re-walks briefly shadow the inner graft until
//!   the inner mount's next event. Self-healing, obscure.
//! - A mount whose host path is missing/unreadable at attach is
//!   silently absent from the tree (no watcher, no graft).

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use dashmap::DashMap;
use objectiveai_sdk::laboratories::filetree::{FileTreeEvent, FileTreeNode};

use crate::host::HostServer;

/// All live mount watches, keyed by canonical host path. A field on
/// [`HostServer`]; empty when no started laboratory has mounts.
#[derive(Default)]
pub struct MountRegistry {
    watches: DashMap<PathBuf, Arc<MountWatch>>,
}

/// One watched host path: its notify watcher, its materialized
/// mount-space tree, and the (laboratory, container mountpoint)
/// subscriptions the pump fans events out to.
pub struct MountWatch {
    /// Canonical host path — the watch root events are relativized
    /// against. For a file mount, the FILE itself (its parent is what
    /// notify watches).
    host_path: PathBuf,
    is_file: bool,
    /// `lab id → set of container mountpoint component-paths` (a lab
    /// can mount the same host dir at several container paths).
    subscribers: std::sync::Mutex<HashMap<String, HashSet<Vec<String>>>>,
    /// Materialized mount-space tree: a dir mount's child nodes, or a
    /// file mount's 0/1 node. The resync currency for grafts.
    tree: std::sync::Mutex<Vec<FileTreeNode>>,
    /// Initial-walk latch — attachers await it before their graft.
    init: tokio::sync::OnceCell<()>,
    /// Held for the watch's life; `take()` at teardown unregisters
    /// the OS watches (dropping the watcher closes the event channel,
    /// which ends the pump).
    watcher: std::sync::Mutex<Option<notify::RecommendedWatcher>>,
    /// The event receiver, parked here between watcher creation
    /// (arm-before-walk) and pump spawn (after the initial walk).
    events: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<notify::Result<notify::Event>>>>,
    pump: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl MountRegistry {
    /// Subscribe `lab_id` (mounting `host_path` at the container path
    /// whose components are `mountpoint`) — creating the watcher on
    /// the first subscription for this host path, and grafting the
    /// mount's tree into the lab's filetree once the initial walk
    /// completes. Idempotent per (lab, host path, mountpoint).
    pub fn attach(
        &self,
        host: &Arc<HostServer>,
        lab_id: &str,
        host_path: &str,
        mountpoint: Vec<String>,
    ) {
        if mountpoint.is_empty() {
            return;
        }
        // Canonicalize ONCE and use the canonical form as both the
        // dedupe key and the watch root: notify's backends echo event
        // paths under the root as given, so strip_prefix stays exact;
        // this also merges case/`\\?\` variants (Windows) and
        // `/tmp` vs `/private/tmp` (macOS). Missing/unreadable host
        // path ⇒ the mount is silently absent (documented punt).
        let Ok(canonical) = std::fs::canonicalize(host_path) else {
            return;
        };
        let Ok(meta) = std::fs::symlink_metadata(&canonical) else {
            return;
        };
        let is_file = !meta.is_dir();

        let watch = {
            let entry = self.watches.entry(canonical.clone());
            let entry = entry.or_insert_with(|| {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                // Sync callback on notify's own thread; unbounded_send
                // never blocks. A closed receiver (watch torn down)
                // just drops events.
                let watcher = notify::recommended_watcher(
                    move |res: notify::Result<notify::Event>| {
                        let _ = tx.send(res);
                    },
                )
                .ok();
                let watcher = watcher.and_then(|mut w| {
                    use notify::Watcher;
                    // Arm BEFORE the initial walk — events during the
                    // walk buffer in the channel and replay as
                    // idempotent re-folds once the pump starts.
                    let (target, mode) = if is_file {
                        (
                            canonical.parent().unwrap_or(&canonical).to_path_buf(),
                            notify::RecursiveMode::NonRecursive,
                        )
                    } else {
                        (canonical.clone(), notify::RecursiveMode::Recursive)
                    };
                    w.watch(&target, mode).ok().map(|_| w)
                });
                Arc::new(MountWatch {
                    host_path: canonical.clone(),
                    is_file,
                    subscribers: std::sync::Mutex::new(HashMap::new()),
                    tree: std::sync::Mutex::new(Vec::new()),
                    init: tokio::sync::OnceCell::new(),
                    watcher: std::sync::Mutex::new(watcher),
                    events: std::sync::Mutex::new(Some(rx)),
                    pump: std::sync::Mutex::new(None),
                })
            });
            // Register the subscriber BEFORE releasing the map entry
            // guard: a concurrent detach's zero-check is serialized
            // against this by the shard lock, and any Snapshot re-graft
            // ordered after our graft necessarily sees the
            // registration.
            entry
                .subscribers
                .lock()
                .expect("subscribers lock")
                .entry(lab_id.to_string())
                .or_default()
                .insert(mountpoint.clone());
            Arc::clone(entry.value())
        };

        let host = Arc::clone(host);
        let lab_id = lab_id.to_string();
        tokio::spawn(async move {
            watch
                .init
                .get_or_init(|| async {
                    // The initial walk, then the pump. Async work
                    // happens with NO locks held; the tree swap is a
                    // brief sync hold.
                    let walked = walk(&watch).await;
                    *watch.tree.lock().expect("tree lock") = walked;
                    let rx = watch.events.lock().expect("events lock").take();
                    if let Some(rx) = rx {
                        let handle = tokio::spawn(pump(
                            Arc::clone(&host),
                            Arc::clone(&watch),
                            rx,
                        ));
                        *watch.pump.lock().expect("pump lock") = Some(handle);
                    }
                })
                .await;
            host.graft_mount(&lab_id, &mountpoint, &watch).await;
        });
    }

    /// Withdraw ALL of `lab_id`'s subscriptions; a watch left with
    /// zero subscribers is torn down (pump aborted, watcher dropped —
    /// which unregisters the OS watches), so watcher count always
    /// returns to zero with the containers.
    pub fn detach_lab(&self, lab_id: &str) {
        self.watches.retain(|_, watch| {
            let mut subs = watch.subscribers.lock().expect("subscribers lock");
            subs.remove(lab_id);
            if subs.is_empty() {
                if let Some(pump) = watch.pump.lock().expect("pump lock").take() {
                    pump.abort();
                }
                watch.watcher.lock().expect("watcher lock").take();
                false
            } else {
                true
            }
        });
    }

    /// Every graft for `lab_id` — one `(mountpoint components, graft
    /// node)` per subscription. Sync leaf locks only: callable under
    /// `attach_lock` (the Snapshot re-graft path).
    pub fn grafts_for(&self, lab_id: &str) -> Vec<(Vec<String>, FileTreeNode)> {
        let mut out = Vec::new();
        for entry in self.watches.iter() {
            let mountpoints: Vec<Vec<String>> = {
                let subs = entry.subscribers.lock().expect("subscribers lock");
                match subs.get(lab_id) {
                    Some(points) => points.iter().cloned().collect(),
                    None => continue,
                }
            };
            for mountpoint in mountpoints {
                if let Some(node) = entry.graft(&mountpoint) {
                    out.push((mountpoint, node));
                }
            }
        }
        out
    }
}

impl MountWatch {
    /// Whether (lab, mountpoint) is still subscribed — grafts re-check
    /// this under `attach_lock` so a graft can never land after the
    /// lab detached.
    pub fn subscribed(&self, lab_id: &str, mountpoint: &[String]) -> bool {
        self.subscribers
            .lock()
            .expect("subscribers lock")
            .get(lab_id)
            .is_some_and(|points| points.contains(mountpoint))
    }

    /// The graft node for one mountpoint: the mount's current tree
    /// wrapped in a node RENAMED to the mountpoint's basename (host
    /// and container basenames can differ, and `FileTreeEvent::apply`
    /// addresses nodes by name). `None` for a file mount whose file is
    /// currently gone.
    pub fn graft(&self, mountpoint: &[String]) -> Option<FileTreeNode> {
        let name = mountpoint.last()?.clone();
        let tree = self.tree.lock().expect("tree lock");
        if self.is_file {
            tree.first().cloned().map(|node| rename(node, name))
        } else {
            Some(FileTreeNode::Directory {
                name,
                created_at: None,
                modified_at: None,
                created_by: None,
                modified_by: None,
                children: tree.clone(),
            })
        }
    }
}

/// The mount-space initial walk: a dir mount's child nodes, a file
/// mount's 0/1 node.
async fn walk(watch: &MountWatch) -> Vec<FileTreeNode> {
    if watch.is_file {
        build_node(&watch.host_path).await.into_iter().collect()
    } else {
        build_children(&watch.host_path).await
    }
}

/// The per-watch event pump: fold each native event into the
/// mount-space tree FIRST, then fan the lab-space delta out to every
/// subscribing laboratory. Ends when the watcher (and with it the
/// channel sender) is dropped at teardown.
async fn pump(
    host: Arc<HostServer>,
    watch: Arc<MountWatch>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<notify::Result<notify::Event>>,
) {
    while let Some(item) = rx.recv().await {
        match item {
            // Watch error or an explicit rescan flag (FSEvents
            // coalescing / buffer overflow): incremental knowledge is
            // gone — re-walk and re-graft everywhere.
            Err(_) => resync(&host, &watch).await,
            Ok(event)
                if event.flag() == Some(notify::event::Flag::Rescan) =>
            {
                resync(&host, &watch).await;
            }
            Ok(event) => handle_event(&host, &watch, event).await,
        }
    }
}

/// Re-walk the mount and re-graft it into every subscribing lab — the
/// mount-scoped resync (never a whole-lab snapshot).
async fn resync(host: &Arc<HostServer>, watch: &Arc<MountWatch>) {
    let walked = walk(watch).await;
    *watch.tree.lock().expect("tree lock") = walked;
    for (lab, mountpoint) in all_subscriptions(watch) {
        host.graft_mount(&lab, &mountpoint, watch).await;
    }
}

/// Map one notify event to mount-space folds + lab-space emissions,
/// mirroring the in-container mapping: create/modify → stat; present
/// ⇒ `Upserted` (a directory carries its re-walked subtree), gone ⇒
/// `Removed`; remove → `Removed`. Events naming the watch root itself
/// (empty relative path) are skipped for dir mounts; for file mounts
/// only events naming exactly the file count.
async fn handle_event(host: &Arc<HostServer>, watch: &Arc<MountWatch>, event: notify::Event) {
    use notify::EventKind;
    let relevant = matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    );
    if !relevant {
        return;
    }
    for path in event.paths {
        if watch.is_file {
            if path != watch.host_path {
                continue;
            }
            // The file mount's whole state is one node: re-stat and
            // re-graft (rename + upsert-or-remove at the mountpoint
            // itself happens inside the graft).
            let node = build_node(&watch.host_path).await;
            *watch.tree.lock().expect("tree lock") = node.into_iter().collect();
            for (lab, mountpoint) in all_subscriptions(watch) {
                match watch.graft(&mountpoint) {
                    Some(_) => host.graft_mount(&lab, &mountpoint, watch).await,
                    None => {
                        host.mount_event(
                            &lab,
                            FileTreeEvent::Removed {
                                path: mountpoint.clone(),
                            },
                        )
                        .await;
                    }
                }
            }
            continue;
        }
        let Some(rel) = rel_components(&watch.host_path, &path) else {
            continue;
        };
        // A rename's "from" side no longer exists → a failed stat is
        // a removal, same as the container-side mapping.
        let delta = match event.kind {
            EventKind::Remove(_) => FileTreeEvent::Removed { path: rel.clone() },
            _ => match build_node(&path).await {
                Some(node) => FileTreeEvent::Upserted {
                    path: rel.clone(),
                    node,
                },
                None => FileTreeEvent::Removed { path: rel.clone() },
            },
        };
        // Fold into the mount-space tree FIRST (sync, released before
        // any emission), so a graft ordered after this emission is
        // guaranteed to contain it.
        delta
            .clone()
            .apply(&mut watch.tree.lock().expect("tree lock"));
        for (lab, mountpoint) in all_subscriptions(watch) {
            let mut lab_path = mountpoint;
            lab_path.extend(rel.iter().cloned());
            let event = match &delta {
                FileTreeEvent::Upserted { node, .. } => FileTreeEvent::Upserted {
                    path: lab_path,
                    node: node.clone(),
                },
                _ => FileTreeEvent::Removed { path: lab_path },
            };
            host.mount_event(&lab, event).await;
        }
    }
}

/// Snapshot of every (lab, mountpoint) subscription — cloned out so
/// no lock is held across the emission awaits.
fn all_subscriptions(watch: &MountWatch) -> Vec<(String, Vec<String>)> {
    let subs = watch.subscribers.lock().expect("subscribers lock");
    subs.iter()
        .flat_map(|(lab, points)| {
            points
                .iter()
                .map(|p| (lab.clone(), p.clone()))
        })
        .collect()
}

/// The path components of `path` relative to the watch root. `None`
/// for the root itself or anything outside it.
fn rel_components(root: &Path, path: &Path) -> Option<Vec<String>> {
    let rel = path.strip_prefix(root).ok()?;
    if rel.as_os_str().is_empty() {
        return None;
    }
    Some(
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect(),
    )
}

/// Rename a node — grafts address the mountpoint by its CONTAINER
/// basename, whatever the host basename is.
fn rename(node: FileTreeNode, new_name: String) -> FileTreeNode {
    match node {
        FileTreeNode::File {
            size,
            created_at,
            modified_at,
            created_by,
            modified_by,
            ..
        } => FileTreeNode::File {
            name: new_name,
            size,
            created_at,
            modified_at,
            created_by,
            modified_by,
        },
        FileTreeNode::Directory {
            created_at,
            modified_at,
            created_by,
            modified_by,
            children,
            ..
        } => FileTreeNode::Directory {
            name: new_name,
            created_at,
            modified_at,
            created_by,
            modified_by,
            children,
        },
        FileTreeNode::Symlink {
            target,
            created_at,
            modified_at,
            created_by,
            modified_by,
            ..
        } => FileTreeNode::Symlink {
            name: new_name,
            target,
            created_at,
            modified_at,
            created_by,
            modified_by,
        },
    }
}

/// Build the [`FileTreeNode`] for a single host path (symlink-aware —
/// links and Windows junctions are leaves, never followed; a directory
/// carries its whole re-walked subtree). `None` when the path is gone.
/// Attribution fields stay `None` — mounts have no agent attribution.
async fn build_node(path: &Path) -> Option<FileTreeNode> {
    let meta = tokio::fs::symlink_metadata(path).await.ok()?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ft = meta.file_type();
    if ft.is_dir() {
        Some(dir_node(name, &meta, build_children(path).await))
    } else {
        Some(leaf_node(path, name, ft.is_symlink(), &meta))
    }
}

/// Build the immediate children of a host directory, recursing into
/// subdirectories. Boxed because async recursion needs an indirected
/// future. Entries that fail to stat are skipped.
fn build_children(
    dir: &Path,
) -> Pin<Box<dyn Future<Output = Vec<FileTreeNode>> + Send + '_>> {
    Box::pin(async move {
        let mut children = Vec::new();
        let mut read = match tokio::fs::read_dir(dir).await {
            Ok(r) => r,
            Err(_) => return children,
        };
        while let Ok(Some(entry)) = read.next_entry().await {
            let path = entry.path();
            // `symlink_metadata` so the KIND reflects the link itself
            // (never followed) — junction cycles can't recurse.
            let Ok(meta) = tokio::fs::symlink_metadata(&path).await else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let ft = meta.file_type();
            let child = if ft.is_dir() {
                dir_node(name, &meta, build_children(&path).await)
            } else {
                leaf_node(&path, name, ft.is_symlink(), &meta)
            };
            children.push(child);
        }
        children
    })
}

fn leaf_node(
    path: &Path,
    name: String,
    is_symlink: bool,
    meta: &std::fs::Metadata,
) -> FileTreeNode {
    let created_at = unix_secs(meta.created().ok());
    let modified_at = unix_secs(meta.modified().ok());
    if is_symlink {
        FileTreeNode::Symlink {
            name,
            target: std::fs::read_link(path)
                .ok()
                .map(|t| t.to_string_lossy().into_owned()),
            created_at,
            modified_at,
            created_by: None,
            modified_by: None,
        }
    } else {
        FileTreeNode::File {
            name,
            size: Some(meta.len()),
            created_at,
            modified_at,
            created_by: None,
            modified_by: None,
        }
    }
}

fn dir_node(
    name: String,
    meta: &std::fs::Metadata,
    children: Vec<FileTreeNode>,
) -> FileTreeNode {
    FileTreeNode::Directory {
        name,
        created_at: unix_secs(meta.created().ok()),
        modified_at: unix_secs(meta.modified().ok()),
        created_by: None,
        modified_by: None,
        children,
    }
}

fn unix_secs(time: Option<std::time::SystemTime>) -> Option<i64> {
    time?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}
