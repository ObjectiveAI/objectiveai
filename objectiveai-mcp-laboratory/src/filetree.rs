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

use std::path::{Path, PathBuf};
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
use objectiveai_sdk::laboratories::filetree::{
    FileKind, FileTreeEntry, FileTreeEvent, FileTreeNode,
};
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
    if let Err(e) = watcher.watch(&root, notify::RecursiveMode::Recursive) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("watch {}: {e}", root.display()),
        )
            .into_response();
    }

    // Build the recursive snapshot off the async runtime.
    let walk_root = root.clone();
    let snapshot_root =
        match tokio::task::spawn_blocking(move || build_tree(&walk_root)).await {
            Ok(node) => node,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("walk join: {e}"),
                )
                    .into_response();
            }
        };

    // The SSE body: snapshot first, then each notify event mapped to a
    // delta. The `watcher` is moved into the stream's closure state so
    // it lives exactly as long as the connection — dropping the
    // response drops the watcher and unregisters the inotify watches.
    let snapshot_event = sse_event(&FileTreeEvent::Snapshot { root: snapshot_root });
    let deltas = rx
        .map(move |res| {
            // Keep `watcher` alive for the stream's lifetime.
            let _keep = &watcher;
            res
        })
        .flat_map(move |res| {
            let events = match res {
                Ok(event) => events_to_deltas(&root, event),
                Err(_) => Vec::new(),
            };
            futures::stream::iter(events.into_iter().map(|e| Ok::<Event, std::convert::Infallible>(e)))
        });
    let stream = futures::stream::once(async move { Ok(snapshot_event) }).chain(deltas);

    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

/// Serialize a [`FileTreeEvent`] into an SSE data frame.
fn sse_event(event: &FileTreeEvent) -> Event {
    Event::default().data(serde_json::to_string(event).unwrap_or_default())
}

/// Map one notify event to zero or more filetree deltas. A create /
/// modify / rename-to stats the path into an `Upserted`; a remove /
/// rename-from emits `Removed`. Paths outside `root` are ignored.
fn events_to_deltas(root: &Path, event: notify::Event) -> Vec<Event> {
    use notify::EventKind;
    let mut out = Vec::new();
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in event.paths {
                // A rename's "from" side no longer exists → treat a
                // failed stat as a removal.
                match entry_for(root, &path) {
                    Some(entry) => {
                        out.push(sse_event(&FileTreeEvent::Upserted { entry }));
                    }
                    None => {
                        if let Some(rel) = rel_path(root, &path) {
                            out.push(sse_event(&FileTreeEvent::Removed { path: rel }));
                        }
                    }
                }
            }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                if let Some(rel) = rel_path(root, &path) {
                    out.push(sse_event(&FileTreeEvent::Removed { path: rel }));
                }
            }
        }
        // Access / other: no tree change.
        _ => {}
    }
    out
}

/// Path relative to `root`, `/`-separated. `None` if `path` is `root`
/// itself or not under it.
fn rel_path(root: &Path, path: &Path) -> Option<String> {
    let rel = pathdiff::diff_paths(path, root)?;
    if rel.as_os_str().is_empty() || rel.starts_with("..") {
        return None;
    }
    Some(
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// Stat a single path into a flat [`FileTreeEntry`] (relative to
/// `root`). `None` when the path is gone or not under `root`.
fn entry_for(root: &Path, path: &Path) -> Option<FileTreeEntry> {
    let rel = rel_path(root, path)?;
    let meta = std::fs::symlink_metadata(path).ok()?;
    let kind = file_kind(&meta.file_type());
    Some(FileTreeEntry {
        path: rel,
        kind,
        size: (kind == FileKind::File).then(|| meta.len()),
        created_at: unix_secs(meta.created().ok()),
        modified_at: unix_secs(meta.modified().ok()),
        // Reserved for the attribution engine.
        created_by: None,
        modified_by: None,
    })
}

/// Recursively build the tree rooted at `root`. The root node's `name`
/// is the watched path; children carry their basenames. Symlinks are
/// leaves (never followed); unreadable entries are skipped.
fn build_tree(root: &Path) -> FileTreeNode {
    let meta = std::fs::symlink_metadata(root).ok();
    let mut node = node_for(
        root.to_string_lossy().into_owned(),
        FileKind::Dir,
        meta.as_ref(),
    );
    node.children = Some(build_children(root));
    node
}

/// Build the immediate children of a directory, recursing into
/// subdirectories. Entries that fail to stat are skipped.
fn build_children(dir: &Path) -> Vec<FileTreeNode> {
    let mut children = Vec::new();
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return children,
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata().or_else(|_| std::fs::symlink_metadata(&path))
        else {
            continue;
        };
        // `entry.metadata()` follows symlinks; re-stat without follow
        // so the KIND reflects the link itself.
        let ft = match std::fs::symlink_metadata(&path) {
            Ok(m) => m.file_type(),
            Err(_) => meta.file_type(),
        };
        let kind = file_kind(&ft);
        let name = entry.file_name().to_string_lossy().into_owned();
        let mut child = node_for(name, kind, Some(&meta));
        if kind == FileKind::Dir {
            child.children = Some(build_children(&path));
        }
        children.push(child);
    }
    children
}

/// Build a node from a name, kind, and optional metadata.
fn node_for(name: String, kind: FileKind, meta: Option<&std::fs::Metadata>) -> FileTreeNode {
    FileTreeNode {
        name,
        kind,
        size: match (kind, meta) {
            (FileKind::File, Some(m)) => Some(m.len()),
            _ => None,
        },
        created_at: meta.and_then(|m| unix_secs(m.created().ok())),
        modified_at: meta.and_then(|m| unix_secs(m.modified().ok())),
        created_by: None,
        modified_by: None,
        children: None,
    }
}

fn file_kind(ft: &std::fs::FileType) -> FileKind {
    if ft.is_symlink() {
        FileKind::Symlink
    } else if ft.is_dir() {
        FileKind::Dir
    } else {
        FileKind::File
    }
}

fn unix_secs(time: Option<std::time::SystemTime>) -> Option<i64> {
    time?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}
