//! `GET /filetree?path=<p>` — a VSCode-style filesystem watch over SSE.
//!
//! The endpoint yields a full recursive tree **snapshot** as its first
//! event, then live **upsert/remove deltas** as the container
//! filesystem changes (inotify via `notify`). A consumer folds the
//! snapshot into a `path → entry` map and applies deltas by path (see
//! the SDK's `objectiveai_sdk::laboratories::filetree::FileTree`).
//!
//! The tree is always rooted at `/` — the whole container. The wire
//! shapes are the SDK's shared `filetree` types.
//!
//! ## Ignored entries do not exist
//!
//! An ignored entry is invisible to this stream, descendants
//! included: absent from the snapshot, never walked, never watched,
//! and any event whose path falls under one is dropped. The ignore
//! set comes from ONE place — the `OBJECTIVEAI_FILETREE_IGNORE` env
//! (parsed once at startup, [`init_ignore_env`]; the env is fixed for
//! the process lifetime). This module is deliberately naive about
//! what the entries mean or why they were chosen — whoever launches
//! the process decides what should not exist.
//!
//! Colon-separated ABSOLUTE PATHS, nothing fancier: each entry
//! ignores that path and everything under it; entries not starting
//! with `/` are skipped, and so is a literal `/` entry (it would
//! erase the entire tree). Plain prefix matching only — cheap enough
//! to run per walked entry and per event on a large filesystem.
//!
//! Any subtree whose watch registration fails is skipped (its changes
//! just don't stream) instead of failing the endpoint — see
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
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
};
use futures::StreamExt;
use objectiveai_sdk::laboratories::filetree::{FileTreeEvent, FileTreeNode};

/// The parsed ignore set — absolute path prefixes, nothing fancier
/// (see the module docs). Naive by design: this module neither knows
/// nor cares what the entries mean. Set once by [`init_ignore_env`];
/// empty when never initialized (standalone runs with no env).
static IGNORE: std::sync::OnceLock<Vec<PathBuf>> = std::sync::OnceLock::new();

/// Parse `OBJECTIVEAI_FILETREE_IGNORE` (colon-separated absolute
/// paths; anything else skipped) into the process-lifetime ignore
/// set. Idempotent; later calls lose.
pub(crate) fn init_ignore_env(raw: Option<&str>) {
    let _ = IGNORE.set(
        raw.unwrap_or_default()
            .split(':')
            // A literal "/" would erase the entire tree — skipped.
            .filter(|entry| entry.starts_with('/') && *entry != "/")
            .map(PathBuf::from)
            .collect(),
    );
}

fn ignore() -> &'static [PathBuf] {
    IGNORE.get_or_init(Vec::new)
}

/// Whether `path` does not exist as far as this stream is concerned:
/// under (or equal to) an ignored path. `/` itself can never match
/// (a literal `/` entry is skipped at parse).
fn is_excluded(path: &Path) -> bool {
    ignore().iter().any(|p| path.starts_with(p))
}

/// Whether ANY path under `dir` could be excluded — the cheap
/// pre-check that decides if a subtree is safe for an indiscriminate
/// recursive watch.
fn subtree_may_contain_excluded(dir: &Path) -> bool {
    ignore().iter().any(|p| p.starts_with(dir))
}

/// Register watches for `dir`, resiliently: excluded paths are never
/// watched; a subtree that may contain an excluded path (or whose
/// recursive registration fails) degrades to a non-recursive watch of
/// the directory itself plus a resilient watch per child directory —
/// notify's recursive mode walks everything indiscriminately, so it is
/// only used for exclusion-free subtrees, and one unwatchable corner
/// skips that corner instead of killing the whole stream. Only a
/// failure to watch `dir` itself NON-recursively is an error.
fn watch_resilient(
    watcher: &mut dyn notify::Watcher,
    dir: &Path,
) -> notify::Result<()> {
    if is_excluded(dir) {
        return Ok(());
    }
    if !subtree_may_contain_excluded(dir)
        && watcher.watch(dir, notify::RecursiveMode::Recursive).is_ok()
    {
        return Ok(());
    }
    watcher.watch(dir, notify::RecursiveMode::NonRecursive)?;
    let Ok(entries) = std::fs::read_dir(dir) else {
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

/// `GET /filetree` — snapshot-then-deltas SSE stream over the whole
/// container filesystem.
pub async fn filetree() -> Response {
    let root = PathBuf::from("/");

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

    Sse::new(stream).into_response()
}

/// Serialize a [`FileTreeEvent`] into an SSE data frame.
fn sse_event(event: &FileTreeEvent) -> Event {
    Event::default().data(serde_json::to_string(event).unwrap_or_default())
}

/// Map one notify event to zero or more filetree deltas. A create /
/// modify / rename-to builds the node at the path into an `Upserted`
/// (a directory re-walks its whole subtree, so a moved-in populated
/// dir arrives as ONE `Upserted` with its contents); a remove /
/// rename-from emits `Removed`. Paths outside `root` — and excluded
/// paths, which do not exist for this stream (an excluded directory's
/// parent IS watched, so events naming the directory itself do fire)
/// — are ignored.
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
                if is_excluded(&path) {
                    continue;
                }
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
                if is_excluded(&path) {
                    continue;
                }
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
/// directory carries its whole re-walked subtree, excluded paths
/// omitted). `None` when the path is gone.
async fn build_node(path: &Path) -> Option<FileTreeNode> {
    let meta = tokio::fs::symlink_metadata(path).await.ok()?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ft = meta.file_type();
    if ft.is_dir() {
        Some(dir_node(path, name, &meta, build_children(path).await))
    } else {
        Some(leaf_node(path, name, ft.is_symlink(), &meta))
    }
}

/// Build the immediate children of a directory, recursing into
/// subdirectories. Boxed because async recursion needs an indirected
/// future. Entries that fail to stat are skipped; excluded paths do
/// not exist (see the module docs).
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
            if is_excluded(&path) {
                continue;
            }
            // `symlink_metadata` so the KIND reflects the link itself
            // (never followed).
            let Ok(meta) = tokio::fs::symlink_metadata(&path).await else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let ft = meta.file_type();
            let child = if ft.is_dir() {
                dir_node(&path, name, &meta, build_children(&path).await)
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
            // The raw link contents — possibly relative, possibly
            // dangling; never resolved.
            target: std::fs::read_link(path)
                .ok()
                .map(|t| t.to_string_lossy().into_owned()),
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
