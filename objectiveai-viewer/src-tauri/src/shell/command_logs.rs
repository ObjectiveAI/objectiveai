//! The daemon's `/listen` broadcast, captured to disk. RUST holds the
//! stream (one resident task, reconnecting for the viewer's life —
//! no webview involved), and splits it into the two-level layout the
//! command-logs tab reads:
//!
//! - `<dir>/state/<state>/viewer/command-logs/<start>.jsonl` — one
//!   line per REQUEST announcement: the broadcast id, the command
//!   path, the producer's FULL identity (agent fields, plugin trio,
//!   the `task` bool), and the request itself. This is the root list
//!   the tab shows.
//! - `<dir>/.../command-logs/<start>/<id>.jsonl` — every response
//!   item for that request (and its `end` terminator), one file per
//!   broadcast id. Clicking a request opens a tab that reads exactly
//!   one of these.
//!
//! Same flow discipline as `viewer-logs`: each line is appended then
//! emitted (`command-logs://request` / `command-logs://item`), and
//! history pulls stream the files BACKWARDS — newest first — so
//! pulls PREPEND while live events APPEND, interleaving safely on
//! seq. The JS side owns memory bounds; Rust holds per-request O(1)
//! bookkeeping (watermark + seq, handle closed once the stream
//! ends).
//!
//! Frame vocabulary (sdk `cli/broadcast_listener/wire.rs`): a
//! request announcement always carries `task` (daemon-authored,
//! always serialized), an item is a bare `{id, value}`, the
//! terminator is `{id, end: true}` — which is how the resident task
//! discriminates without per-leaf typing.

use std::collections::HashMap;
use std::path::PathBuf;

use futures::StreamExt;
use tauri::{Emitter, Manager};

use super::jsonl::{JsonlFile, now_ms, pull_backwards};

/// Default (and maximum) entries one pull streams.
const PULL_DEFAULT: u64 = 1000;

/// One request announcement, as logged and as listed by the tab.
/// Identity fields ride verbatim from the wire — the whole point is
/// seeing WHO ran the command.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestEntry {
    /// Monotonic within one viewer run — the list's ordering key.
    pub seq: u64,
    /// Epoch millis, stamped on receipt.
    pub at_ms: u64,
    /// The broadcast stream id — the items file's name, and the JS
    /// upsert key.
    pub id: String,
    /// The command path (the request's `path_type`), when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_instance_hierarchy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_full_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_remote: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_ids: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_version: Option<String>,
    /// Fired by the task scheduler.
    #[serde(default)]
    pub task: bool,
    /// The run's actual request, verbatim.
    pub request: serde_json::Value,
}

/// One line of a request's items file: a response item, or the
/// stream's `end` terminator.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ItemEntry {
    /// Monotonic within one request's stream — the JS upsert key.
    pub seq: u64,
    /// Epoch millis, stamped on receipt.
    pub at_ms: u64,
    /// The response item, verbatim (`None` on the end terminator).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// The stream ended — exactly one, last.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub end: bool,
}

/// The live event payload for one item — the request id routes it to
/// the right tab (the pane filters; items files know their id from
/// their name, so the line itself doesn't carry it).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ItemEvent {
    pub request_id: String,
    pub item: ItemEntry,
}

/// The managed sink for this viewer RUN: the root requests file plus
/// per-request items files, all under the run's timestamp.
pub struct CommandLogSink {
    inner: tokio::sync::Mutex<CmdInner>,
}

struct CmdInner {
    /// The per-run items directory (`command-logs/<start>/`).
    items_dir: PathBuf,
    root: JsonlFile,
    next_seq: u64,
    /// Per-request bookkeeping, keyed by broadcast id. Entries are
    /// NEVER removed during a run (seq continuity for late frames);
    /// the OS handle is dropped on `end` instead.
    streams: HashMap<String, ItemsFile>,
}

struct ItemsFile {
    file: JsonlFile,
    next_seq: u64,
}

impl CommandLogSink {
    pub fn new(command_logs_dir: PathBuf) -> Self {
        let stamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        Self {
            inner: tokio::sync::Mutex::new(CmdInner {
                items_dir: command_logs_dir.join(&stamp),
                root: JsonlFile::new(command_logs_dir.join(format!("{stamp}.jsonl"))),
                next_seq: 0,
                streams: HashMap::new(),
            }),
        }
    }
}

impl CmdInner {
    /// The items bookkeeping for `id`, created on first sight (also
    /// covers items whose announcement predated this viewer run's
    /// connect — their file exists without a root row).
    fn items(&mut self, id: &str) -> &mut ItemsFile {
        let items_dir = &self.items_dir;
        self.streams
            .entry(id.to_string())
            .or_insert_with(|| ItemsFile {
                file: JsonlFile::new(items_dir.join(format!("{id}.jsonl"))),
                next_seq: 0,
            })
    }
}

/// A broadcast id that is safe to use as a file name. The daemon
/// mints uuids, but the id crosses a trust boundary twice (the wire
/// in, the pull command back in from JS) — never let one traverse
/// paths.
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Spawn the resident `/listen` capture: connect, consume, append +
/// emit; on ANY disconnect, retry after a beat, forever — the viewer
/// may well outlive several daemons.
pub fn spawn_command_listener(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            listen_once(&app).await;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

async fn listen_once(app: &tauri::AppHandle) {
    let request = app
        .state::<crate::daemon_proxy::DaemonProxy>()
        .listen_builder();
    let Ok(response) = request.send().await else {
        return;
    };
    if !response.status().is_success() {
        return;
    }
    use eventsource_stream::Eventsource;
    let mut events = response.bytes_stream().eventsource();
    while let Some(Ok(event)) = events.next().await {
        handle_frame(app, &event.data).await;
    }
}

/// One `/listen` frame: discriminate (end marker → request's `task`
/// marker → bare item), append to the right file, emit the live
/// event.
async fn handle_frame(app: &tauri::AppHandle, data: &str) {
    let Ok(frame) = serde_json::from_str::<serde_json::Value>(data) else {
        return;
    };
    let Some(id) = frame.get("id").and_then(|v| v.as_str()).map(str::to_string) else {
        return;
    };
    if !safe_id(&id) {
        return;
    }
    let sink = app.state::<CommandLogSink>();
    let at_ms = now_ms();

    if frame.get("end").and_then(|v| v.as_bool()) == Some(true) {
        let item = {
            let mut inner = sink.inner.lock().await;
            let items = inner.items(&id);
            items.next_seq += 1;
            let item = ItemEntry {
                seq: items.next_seq,
                at_ms,
                value: None,
                end: true,
            };
            items.file.append(&item).await;
            items.file.close_handle();
            item
        };
        let _ = app.emit("command-logs://item", &ItemEvent { request_id: id, item });
        return;
    }

    // A request announcement always carries the daemon-authored
    // `task` bool; a bare item never does.
    if frame.get("task").is_some() {
        let str_field = |key: &str| {
            frame
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        let request = frame.get("value").cloned().unwrap_or(serde_json::Value::Null);
        let entry = {
            let mut inner = sink.inner.lock().await;
            inner.next_seq += 1;
            let entry = RequestEntry {
                seq: inner.next_seq,
                at_ms,
                id: id.clone(),
                path: request
                    .get("path_type")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                agent_instance_hierarchy: str_field("agent_instance_hierarchy"),
                agent_id: str_field("agent_id"),
                agent_full_id: str_field("agent_full_id"),
                agent_remote: str_field("agent_remote"),
                response_id: str_field("response_id"),
                response_ids: str_field("response_ids"),
                plugin_owner: str_field("plugin_owner"),
                plugin_name: str_field("plugin_name"),
                plugin_version: str_field("plugin_version"),
                task: frame.get("task").and_then(|v| v.as_bool()).unwrap_or(false),
                request,
            };
            inner.root.append(&entry).await;
            entry
        };
        let _ = app.emit("command-logs://request", &entry);
        return;
    }

    let Some(value) = frame.get("value").cloned() else {
        return;
    };
    let item = {
        let mut inner = sink.inner.lock().await;
        let items = inner.items(&id);
        items.next_seq += 1;
        let item = ItemEntry {
            seq: items.next_seq,
            at_ms,
            value: Some(value),
            end: false,
        };
        items.file.append(&item).await;
        item
    };
    let _ = app.emit("command-logs://item", &ItemEvent { request_id: id, item });
}

/// Stream this run's REQUEST list backwards — newest first — through
/// `on_entry`, up to `count` (default/cap [`PULL_DEFAULT`]). Same
/// contract as `logs_pull`: pulls prepend history, the
/// `command-logs://request` events append the present.
#[tauri::command]
pub async fn command_logs_pull(
    sink: tauri::State<'_, CommandLogSink>,
    count: Option<u64>,
    on_entry: tauri::ipc::Channel<RequestEntry>,
) -> Result<(), String> {
    let (path, len) = {
        let inner = sink.inner.lock().await;
        (inner.root.path().to_path_buf(), inner.root.written_len())
    };
    let max = count.unwrap_or(PULL_DEFAULT).min(PULL_DEFAULT);
    pull_backwards::<RequestEntry, _>(&path, len, max, |entry| on_entry.send(entry).is_ok()).await
}

/// Stream ONE request's items backwards — newest first — through
/// `on_item`, up to `count` (default/cap [`PULL_DEFAULT`]).
#[tauri::command]
pub async fn command_log_items_pull(
    sink: tauri::State<'_, CommandLogSink>,
    id: String,
    count: Option<u64>,
    on_item: tauri::ipc::Channel<ItemEntry>,
) -> Result<(), String> {
    if !safe_id(&id) {
        return Err("invalid request id".to_string());
    }
    let (path, len) = {
        let mut inner = sink.inner.lock().await;
        let items = inner.items(&id);
        (items.file.path().to_path_buf(), items.file.written_len())
    };
    let max = count.unwrap_or(PULL_DEFAULT).min(PULL_DEFAULT);
    pull_backwards::<ItemEntry, _>(&path, len, max, |item| on_item.send(item).is_ok()).await
}
