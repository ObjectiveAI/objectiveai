//! Host-side mount watching: the native event source behind each
//! laboratory's mount entries in its [`crate::lab_tree::LabTree`].
//!
//! Mounted host folders are invisible to the in-container `/filetree`
//! stream (the mount protocol delivers no change events and walks far
//! slower than native, so the MCP is told to ignore them). This
//! module watches each mounted host path NATIVELY — inotify on Linux,
//! FSEvents on macOS, ReadDirectoryChangesW on Windows, all via the
//! `notify` crate — and feeds the per-lab trees through the host's
//! `source_mount_*` ingestion: a DELIVERY when a walk completes
//! ([`MountWatch::ready`] → `source_mount_delivered`) and a DELTA per
//! filesystem event (`source_mount_delta`). The completeness /
//! emission policy lives entirely in [`crate::lab_tree`] — this module
//! never decides what a laboratory sees, only what a mount contains.
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
//! The pump folds `tree` BEFORE emitting, and compose reads `tree`
//! under one `attach_lock` hold — so any composed snapshot ordered
//! after a delta already contains that delta. The walk task's strict
//! order is: store tree → set the `walked` latch → spawn the pump —
//! so a delta can never observe an unwalked tree, and a delivery can
//! never announce one.
//!
//! ## Known punts
//!
//! - Nested mounts from DIFFERENT host dirs (`/w/a` + `/w/a/b`): the
//!   outer watcher's re-walks briefly shadow the inner graft until
//!   the inner mount's next event. Self-healing, obscure.
//! - A mount whose host path is missing/unreadable at attach is
//!   silently absent from the tree (no watcher, no source — the lab's
//!   tree completes without it).

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
    /// file mount's 0/1 node. Read live by `LabTree::compose`.
    tree: std::sync::Mutex<Vec<FileTreeNode>>,
    /// The walk latch: flips true once the initial walk has stored the
    /// tree. [`Self::snapshot_node`] refuses to represent an unwalked
    /// mount; deliveries await [`Self::ready`].
    walked: tokio::sync::watch::Sender<bool>,
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
    /// whose components are `mountpoint`) — creating the watcher and
    /// spawning its one walk task on the first subscription for this
    /// host path. Returns the watch for [`crate::lab_tree::LabTree`]
    /// source registration; `None` when the host path is
    /// missing/unreadable (the mount is then simply absent). Does NOT
    /// announce delivery — the caller spawns the `ready()` →
    /// `source_mount_delivered` notifier AFTER registering the lab's
    /// source set, so a delivery can never race its own registration.
    /// Idempotent per (lab, host path, mountpoint).
    pub async fn attach(
        &self,
        host: &Arc<HostServer>,
        lab_id: &str,
        host_path: &str,
        mountpoint: Vec<String>,
    ) -> Option<Arc<MountWatch>> {
        if mountpoint.is_empty() {
            return None;
        }
        // Canonicalize ONCE and use the canonical form as both the
        // dedupe key and the watch root: notify's backends echo event
        // paths under the root as given, so strip_prefix stays exact;
        // this also merges case/`\\?\` variants (Windows) and
        // `/tmp` vs `/private/tmp` (macOS).
        let canonical = tokio::fs::canonicalize(host_path).await.ok()?;
        let meta = tokio::fs::symlink_metadata(&canonical).await.ok()?;
        let is_file = !meta.is_dir();

        let mut created = false;
        let watch = {
            let entry = self.watches.entry(canonical.clone());
            let entry = entry.or_insert_with(|| {
                created = true;
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
                    walked: tokio::sync::watch::channel(false).0,
                    watcher: std::sync::Mutex::new(watcher),
                    events: std::sync::Mutex::new(Some(rx)),
                    pump: std::sync::Mutex::new(None),
                })
            });
            // Register the subscriber BEFORE releasing the map entry
            // guard: a concurrent detach's zero-check is serialized
            // against this by the shard lock.
            entry
                .subscribers
                .lock()
                .expect("subscribers lock")
                .entry(lab_id.to_string())
                .or_default()
                .insert(mountpoint);
            Arc::clone(entry.value())
        };

        if created {
            // Exactly-once per watch instance: only the vacant arm
            // spawns the walk. Strict order inside: store tree → set
            // latch → spawn pump.
            let host = Arc::clone(host);
            let walk_watch = Arc::clone(&watch);
            tokio::spawn(async move {
                let walked = walk(&walk_watch).await;
                *walk_watch.tree.lock().expect("tree lock") = walked;
                let _ = walk_watch.walked.send_replace(true);
                let rx = walk_watch.events.lock().expect("events lock").take();
                if let Some(rx) = rx {
                    let handle =
                        tokio::spawn(pump(host, Arc::clone(&walk_watch), rx));
                    *walk_watch.pump.lock().expect("pump lock") = Some(handle);
                }
            });
        }
        Some(watch)
    }

    /// Withdraw ALL of `lab_id`'s subscriptions; a watch left with
    /// zero subscribers is torn down (pump aborted, watcher dropped —
    /// which unregisters the OS watches), so watcher count always
    /// returns to zero with the containers. A torn-down watch's cached
    /// tree stays readable through any `Arc` a `LabTree` still holds —
    /// that is the frozen view.
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
}

impl MountWatch {
    /// Resolves once the initial walk has stored the tree. Deliveries
    /// await this before announcing to a lab's source set.
    pub async fn ready(&self) {
        let mut rx = self.walked.subscribe();
        let _ = rx.wait_for(|walked| *walked).await;
    }

    /// The node representing this mount at `mountpoint` — its current
    /// cached tree wrapped in a node RENAMED to the mountpoint's
    /// basename (host and container basenames can differ; the shared
    /// fold addresses nodes by name). `None` for an unwalked watch
    /// (unrepresentable — compose only runs on delivered sources, and
    /// delivery implies walked) and for a file mount whose file is
    /// currently gone (absence is its truthful state). Sync leaf lock
    /// only: callable under `attach_lock`.
    pub fn snapshot_node(&self, mountpoint: &[String]) -> Option<FileTreeNode> {
        if !*self.walked.borrow() {
            return None;
        }
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

/// The mount-space walk: a dir mount's child nodes, a file mount's
/// 0/1 node.
async fn walk(watch: &MountWatch) -> Vec<FileTreeNode> {
    if watch.is_file {
        build_node(watch.host_path.clone())
            .await
            .into_iter()
            .collect()
    } else {
        build_children(&watch.host_path).await
    }
}

/// Build one node (with its whole subtree for a directory —
/// concurrently, like the walk it is in miniature). `None` when the
/// path is gone. Symlinks and Windows junctions are leaves, never
/// followed. Attribution fields stay `None` — mounts have no agent
/// attribution.
async fn build_node(path: PathBuf) -> Option<FileTreeNode> {
    let meta = tokio::fs::symlink_metadata(&path).await.ok()?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ft = meta.file_type();
    if ft.is_dir() {
        Some(dir_node(name, &meta, build_children(&path).await))
    } else {
        Some(leaf_node(&path, name, ft.is_symlink(), &meta).await)
    }
}

/// The per-watch event pump: fold each native event into the
/// mount-space tree FIRST, then fan the lab-space delta out to every
/// subscribing laboratory via the host's source ingestion. Ends when
/// the watcher (and with it the channel sender) is dropped at
/// teardown.
async fn pump(
    host: Arc<HostServer>,
    watch: Arc<MountWatch>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<notify::Result<notify::Event>>,
) {
    while let Some(item) = rx.recv().await {
        match item {
            // Watch error or an explicit rescan flag (FSEvents
            // coalescing / buffer overflow): incremental knowledge is
            // gone — re-walk and re-deliver everywhere (each delivery
            // re-composes that lab's snapshot).
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

/// Re-walk the mount and re-deliver it to every subscribing lab — the
/// mount-scoped resync (each delivery re-composes; never a raw wipe).
async fn resync(host: &Arc<HostServer>, watch: &Arc<MountWatch>) {
    let walked = walk(watch).await;
    *watch.tree.lock().expect("tree lock") = walked;
    for (lab, mountpoint) in all_subscriptions(watch) {
        host.source_mount_delivered(&lab, &mountpoint, watch).await;
    }
}

/// Map one notify event to a mount-space fold + lab-space source
/// deltas, mirroring the in-container mapping: create/modify → stat;
/// present ⇒ `Upserted` (a directory carries its re-walked subtree),
/// gone ⇒ `Removed`; remove → `Removed`. Events naming the watch root
/// itself (empty relative path) are skipped for dir mounts; for file
/// mounts only events naming exactly the file count, and the whole
/// mount state re-delivers as one node at the mountpoint.
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
            let node = build_node(watch.host_path.clone()).await;
            *watch.tree.lock().expect("tree lock") = node.into_iter().collect();
            for (lab, mountpoint) in all_subscriptions(watch) {
                let delta = match watch.snapshot_node(&mountpoint) {
                    Some(node) => FileTreeEvent::Upserted {
                        path: mountpoint.clone(),
                        node,
                    },
                    None => FileTreeEvent::Removed {
                        path: mountpoint.clone(),
                    },
                };
                host.source_mount_delta(&lab, &mountpoint, delta).await;
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
            _ => match build_node(path.clone()).await {
                Some(node) => FileTreeEvent::Upserted {
                    path: rel.clone(),
                    node,
                },
                None => FileTreeEvent::Removed { path: rel.clone() },
            },
        };
        // Fold into the mount-space tree FIRST (sync, released before
        // any emission), so any composed snapshot ordered after this
        // emission is guaranteed to contain it.
        delta
            .clone()
            .apply(&mut watch.tree.lock().expect("tree lock"));
        for (lab, mountpoint) in all_subscriptions(watch) {
            let mut lab_path = mountpoint.clone();
            lab_path.extend(rel.iter().cloned());
            let event = match &delta {
                FileTreeEvent::Upserted { node, .. } => FileTreeEvent::Upserted {
                    path: lab_path,
                    node: node.clone(),
                },
                _ => FileTreeEvent::Removed { path: lab_path },
            };
            host.source_mount_delta(&lab, &mountpoint, event).await;
        }
    }
}

/// Snapshot of every (lab, mountpoint) subscription — cloned out so
/// no lock is held across the emission awaits.
fn all_subscriptions(watch: &MountWatch) -> Vec<(String, Vec<String>)> {
    let subs = watch.subscribers.lock().expect("subscribers lock");
    subs.iter()
        .flat_map(|(lab, points)| points.iter().map(|p| (lab.clone(), p.clone())))
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

/// Rename a node — mount nodes are addressed by their CONTAINER
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

/// Build the immediate children of a host directory, recursing into
/// subdirectories. MAXIMUM PARALLELISM: every entry's stat — and
/// every subdirectory's entire walk — runs concurrently via
/// `join_all`, saturating tokio's blocking pool with filesystem
/// syscalls instead of paying their latency one at a time (`join_all`
/// preserves entry order). Boxed because async recursion needs an
/// indirected future. Entries that fail to stat are skipped.
/// `DirEntry::metadata()` never traverses symlinks.
fn build_children(
    dir: &Path,
) -> Pin<Box<dyn Future<Output = Vec<FileTreeNode>> + Send + '_>> {
    Box::pin(async move {
        let Ok(mut read) = tokio::fs::read_dir(dir).await else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        while let Ok(Some(entry)) = read.next_entry().await {
            entries.push(entry);
        }
        futures::future::join_all(entries.into_iter().map(|entry| async move {
            let path = entry.path();
            let meta = entry.metadata().await.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let ft = meta.file_type();
            Some(if ft.is_dir() {
                dir_node(name, &meta, build_children(&path).await)
            } else {
                leaf_node(&path, name, ft.is_symlink(), &meta).await
            })
        }))
        .await
        .into_iter()
        .flatten()
        .collect()
    })
}

async fn leaf_node(
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
            target: tokio::fs::read_link(path)
                .await
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
