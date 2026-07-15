//! `GET /filetree?path=<p>` — a VSCode-style filesystem watch over SSE.
//!
//! The endpoint yields a full recursive tree **snapshot** as its first
//! event, then live **upsert/remove deltas** as the container
//! filesystem changes (inotify via `notify`). A consumer folds the
//! snapshot into a `path → entry` map and applies deltas by path (see
//! the SDK's `objectiveai_sdk::laboratories::filetree::FileTree`).
//!
//! `path` defaults to `/` (the whole container). The wire shapes are
//! the SDK's shared `filetree` types.
//!
//! ## Virtual filesystems and unwatchable subtrees
//!
//! Kernel pseudo-filesystems ([`VIRTUAL_ROOTS`]: `/proc`, `/sys`,
//! `/dev`) are NEVER walked into or watched — their magic files abort
//! inotify registration wholesale (`watch /` used to die on
//! `/proc/tty/driver` with EACCES, and one failure killed the whole
//! stream), their contents churn constantly, and they aren't
//! laboratory data. They appear in the tree as childless directories.
//! Any OTHER subtree whose watch registration fails is skipped (its
//! changes just don't stream) instead of failing the endpoint — see
//! [`watch_resilient`].
//!
//! No auth (v1): rides the same loopback-published MCP port the
//! conduit dials, so it's reachable only by the conduit — the same
//! trust model as `/export` / `/import`.
//!
//! ## Snapshot-race correctness
//!
//! The watcher is armed BEFORE the tree walk, feeding an unbounded
//! channel. Events that land during the walk buffer in that channel
//! and replay as deltas the moment forwarding starts — so no change is
//! lost in the window between the snapshot and the live stream.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::UNIX_EPOCH;

use axum::{
    extract::Query,
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::StreamExt;
use notify::Watcher;
use objectiveai_sdk::laboratories::filetree::{FileTreeEvent, FileTreeNode};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PathQuery {
    /// Absolute path inside the container to watch. Defaults to `/`.
    #[serde(default = "default_root")]
    path: String,
}

fn default_root() -> String {
    "/".to_string()
}

/// Kernel pseudo-filesystems — never walked into, never watched (see
/// the module docs). Shown as childless directories.
const VIRTUAL_ROOTS: [&str; 3] = ["/proc", "/sys", "/dev"];

/// Whether `path` IS one of the virtual kernel filesystem roots.
fn is_virtual(path: &Path) -> bool {
    VIRTUAL_ROOTS.iter().any(|root| path == Path::new(root))
}

/// Register watches for `root`, resiliently: virtual roots are never
/// watched; a subtree whose RECURSIVE registration fails degrades to a
/// non-recursive watch of the directory itself plus a resilient watch
/// per child directory (so one unwatchable corner — historically
/// `/proc/tty/driver` under `/` — skips that corner instead of killing
/// the whole stream). Only a failure to watch `root` itself
/// NON-recursively is an error.
fn watch_resilient(
    watcher: &mut dyn notify::Watcher,
    root: &Path,
) -> notify::Result<()> {
    if is_virtual(root) {
        return Ok(());
    }
    if watcher.watch(root, notify::RecursiveMode::Recursive).is_ok() {
        return Ok(());
    }
    watcher.watch(root, notify::RecursiveMode::NonRecursive)?;
    let Ok(entries) = std::fs::read_dir(root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if is_dir {
            // Per-child failures are skipped — that child's changes
            // just don't stream.
            let _ = watch_resilient(watcher, &path);
        }
    }
    Ok(())
}

/// `GET /filetree?path=<p>` — snapshot-then-deltas SSE stream.
pub async fn filetree(Query(q): Query<PathQuery>) -> Response {
    let root = PathBuf::from(&q.path);
    let metadata = match tokio::fs::metadata(&root).await {
        Ok(m) => m,
        Err(e) => {
            return (StatusCode::NOT_FOUND, format!("stat {}: {e}", root.display()))
                .into_response();
        }
    };
    if !metadata.is_dir() {
        return (StatusCode::BAD_REQUEST, "path is not a directory").into_response();
    }

    // Arm the watcher FIRST — events during the walk buffer in the
    // channel and replay when forwarding begins.
    let (tx, rx) = futures::channel::mpsc::unbounded::<notify::Result<notify::Event>>();
    let mut watcher = match notify::recommended_watcher(move |res| {
        // Sync callback on notify's own thread; unbounded_send never
        // blocks. A closed receiver (client gone) just drops events.
        let _ = tx.unbounded_send(res);
    }) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("watch init: {e}"),
            )
                .into_response();
        }
    };
    if let Err(e) = watch_resilient(&mut watcher, &root) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("watch {}: {e}", root.display()),
        )
            .into_response();
    }
    // Shared + mutable for the stream's life: composite-watched
    // regions (a recursive registration that degraded) need watches
    // ADDED for directories created later — see the delta mapping.
    let watcher = std::sync::Arc::new(std::sync::Mutex::new(watcher));

    // Build the recursive snapshot with async fs — no blocking thread
    // parked for the whole walk. The snapshot is the watched root's
    // child nodes (the root's own identity is the requested path).
    let snapshot_children = build_children(&root).await;

    // The SSE body: snapshot first, then each notify event mapped to a
    // delta. The `watcher` is moved into the stream's closure state so
    // it lives exactly as long as the connection — dropping the
    // response drops the watcher and unregisters the inotify watches.
    let snapshot_event = sse_event(&FileTreeEvent::Snapshot {
        children: snapshot_children,
    });
    let deltas = rx
        .then(move |res| {
            // The Arc keeps the watcher (and its inotify registrations)
            // alive for exactly the stream's lifetime.
            let watcher = std::sync::Arc::clone(&watcher);
            let root = root.clone();
            async move {
                match res {
                    Ok(event) => events_to_deltas(&root, event, &watcher).await,
                    // A watch error — most importantly an inotify queue
                    // OVERFLOW (`IN_Q_OVERFLOW`: events were dropped
                    // faster than we drained, so we no longer know
                    // what changed) — is unrecoverable incrementally.
                    // RESYNC: re-walk and emit a fresh snapshot; the
                    // client's `Snapshot` fold replaces its whole tree.
                    // A spurious resync (any other watch error) is
                    // harmless — it replaces the tree with an identical
                    // one.
                    Err(_) => vec![sse_event(&FileTreeEvent::Snapshot {
                        children: build_children(&root).await,
                    })],
                }
            }
        })
        .flat_map(|events| {
            futures::stream::iter(
                events
                    .into_iter()
                    .map(Ok::<Event, std::convert::Infallible>),
            )
        });
    let stream = futures::stream::once(async move { Ok(snapshot_event) }).chain(deltas);

    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

/// Serialize a [`FileTreeEvent`] into an SSE data frame.
fn sse_event(event: &FileTreeEvent) -> Event {
    Event::default().data(serde_json::to_string(event).unwrap_or_default())
}

/// Map one notify event to zero or more filetree deltas. A create /
/// modify / rename-to builds the node at the path into an `Upserted`
/// (a directory re-walks its whole subtree, so a moved-in populated
/// dir arrives as ONE `Upserted` with its contents); a remove /
/// rename-from emits `Removed`. Paths outside `root` are ignored.
///
/// An upserted DIRECTORY also (re)registers a resilient watch on
/// itself: inside a composite-watched region (a recursive
/// registration that degraded — see [`watch_resilient`]) a new
/// directory has no watch of its own; inside a healthy recursive
/// region the extra registration is a harmless duplicate.
async fn events_to_deltas(
    root: &Path,
    event: notify::Event,
    watcher: &std::sync::Arc<std::sync::Mutex<notify::RecommendedWatcher>>,
) -> Vec<Event> {
    use notify::EventKind;
    let mut out = Vec::new();
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in event.paths {
                let Some(components) = rel_components(root, &path) else {
                    continue;
                };
                // A rename's "from" side no longer exists → treat a
                // failed stat as a removal.
                match build_node(&path).await {
                    Some(node) => {
                        if matches!(node, FileTreeNode::Directory { .. })
                            && let Ok(mut watcher) = watcher.lock()
                        {
                            let _ = watch_resilient(&mut *watcher, &path);
                        }
                        out.push(sse_event(&FileTreeEvent::Upserted {
                            path: components,
                            node,
                        }));
                    }
                    None => out.push(sse_event(&FileTreeEvent::Removed {
                        path: components,
                    })),
                }
            }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                if let Some(components) = rel_components(root, &path) {
                    out.push(sse_event(&FileTreeEvent::Removed { path: components }));
                }
            }
        }
        // Access / other: no tree change.
        _ => {}
    }
    out
}

/// The path components relative to `root`. `None` if `path` is `root`
/// itself or not under it.
fn rel_components(root: &Path, path: &Path) -> Option<Vec<String>> {
    let rel = pathdiff::diff_paths(path, root)?;
    if rel.as_os_str().is_empty() || rel.starts_with("..") {
        return None;
    }
    Some(
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect(),
    )
}

/// Build the [`FileTreeNode`] for a single path (symlink-aware; a
/// directory carries its whole re-walked subtree). `None` when the
/// path is gone.
async fn build_node(path: &Path) -> Option<FileTreeNode> {
    let meta = tokio::fs::symlink_metadata(path).await.ok()?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ft = meta.file_type();
    if ft.is_dir() {
        // Virtual kernel filesystems stay childless here too (an
        // attribute change on `/proc` itself can fire off the root's
        // own watch even though its contents are never watched).
        let children = if is_virtual(path) {
            Vec::new()
        } else {
            build_children(path).await
        };
        Some(dir_node(path, name, &meta, children))
    } else {
        Some(leaf_node(path, name, ft.is_symlink(), &meta))
    }
}

/// Build the immediate children of a directory, recursing into
/// subdirectories. Boxed because async recursion needs an indirected
/// future. Entries that fail to stat are skipped.
fn build_children(dir: &Path) -> Pin<Box<dyn Future<Output = Vec<FileTreeNode>> + Send + '_>> {
    Box::pin(async move {
        let mut children = Vec::new();
        let mut read = match tokio::fs::read_dir(dir).await {
            Ok(r) => r,
            Err(_) => return children,
        };
        while let Ok(Some(entry)) = read.next_entry().await {
            let path = entry.path();
            // `symlink_metadata` so the KIND reflects the link itself
            // (never followed).
            let Ok(meta) = tokio::fs::symlink_metadata(&path).await else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let ft = meta.file_type();
            let child = if ft.is_dir() {
                // Virtual kernel filesystems appear as childless
                // directories — never walked (see the module docs).
                let grandchildren = if is_virtual(&path) {
                    Vec::new()
                } else {
                    build_children(&path).await
                };
                dir_node(&path, name, &meta, grandchildren)
            } else {
                leaf_node(&path, name, ft.is_symlink(), &meta)
            };
            children.push(child);
        }
        children
    })
}

/// A `File` or `Symlink` leaf node from a path, name + metadata.
fn leaf_node(
    path: &Path,
    name: String,
    is_symlink: bool,
    meta: &std::fs::Metadata,
) -> FileTreeNode {
    let created_at = unix_secs(meta.created().ok());
    let modified_at = unix_secs(meta.modified().ok());
    let attr = crate::attribution::lookup(path);
    if is_symlink {
        FileTreeNode::Symlink {
            name,
            created_at,
            modified_at,
            created_by: attr.created_by,
            modified_by: attr.modified_by,
        }
    } else {
        FileTreeNode::File {
            name,
            size: Some(meta.len()),
            created_at,
            modified_at,
            created_by: attr.created_by,
            modified_by: attr.modified_by,
        }
    }
}

/// A `Directory` node from a path, name, metadata, and its children.
fn dir_node(
    path: &Path,
    name: String,
    meta: &std::fs::Metadata,
    children: Vec<FileTreeNode>,
) -> FileTreeNode {
    let attr = crate::attribution::lookup(path);
    FileTreeNode::Directory {
        name,
        created_at: unix_secs(meta.created().ok()),
        modified_at: unix_secs(meta.modified().ok()),
        created_by: attr.created_by,
        modified_by: attr.modified_by,
        children,
    }
}

fn unix_secs(time: Option<std::time::SystemTime>) -> Option<i64> {
    time?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}
