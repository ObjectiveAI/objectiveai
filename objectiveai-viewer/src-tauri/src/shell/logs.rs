//! Cross-webview log capture. Every webview — chrome and content
//! alike — gets [`CAPTURE_INIT_SCRIPT`] as an initialization script:
//! it runs at document start, wraps every `console.*` method plus
//! uncaught errors and unhandled rejections, and forwards them to
//! [`logs_report`], buffering until the IPC bootstrap is up. Because
//! it needs NO cooperation from the page's own code, a webview whose
//! bundle fails to boot still reports its death.
//!
//! Storage is a LOGFILE, not memory: Rust stamps each report (seq,
//! time, source), APPENDS one JSONL line to
//! `<dir>/state/<state>/viewer/viewer-logs/<viewer-start>.jsonl`,
//! and emits the entry as `logs://appended`. The viewer-logs tab's
//! boot read is [`logs_pull`], which STREAMS the file backwards —
//! newest first, straight off disk through an IPC channel. That
//! order is what disambiguates the two flows at the consumer:
//! historic pulls PREPEND (ever older), live events APPEND (ever
//! newer), and they interleave safely because everything keys by
//! `seq`. The JS side owns the ring/cap; Rust holds O(1) state and
//! the file survives a viewer crash for post-mortem reading.

use std::path::PathBuf;

use tauri::Emitter;

use super::jsonl::{JsonlFile, pull_backwards};

/// The capture forwarder, injected into every webview builder.
pub const CAPTURE_INIT_SCRIPT: &str = include_str!("capture.js");

/// Default (and maximum) entries a [`logs_pull`] streams.
const PULL_DEFAULT: u64 = 1000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    /// Monotonic within one viewer run, never reused — the upsert
    /// key on the JS side.
    pub seq: u64,
    /// Epoch millis, stamped by RUST on receipt (a webview does not
    /// get to claim its own clock).
    pub at_ms: u64,
    /// Where it came from: a content webview reports as its tab's
    /// TITLE (resolved at receipt), the chrome as `viewer-container`.
    pub source: String,
    /// A console level (`log`/`info`/`warn`/`error`/`debug`/`trace`),
    /// or `uncaught` / `unhandledrejection`.
    pub level: String,
    pub message: String,
    /// Stack trace, when there is one.
    pub detail: Option<String>,
}

/// The managed append-only sink for this viewer RUN — the path
/// carries the start timestamp, so every run appends to its own
/// fresh file and old runs linger on disk for post-mortem reading.
pub struct LogSink {
    inner: tokio::sync::Mutex<SinkInner>,
}

struct SinkInner {
    file: JsonlFile,
    next_seq: u64,
}

impl LogSink {
    pub fn new(viewer_logs_dir: PathBuf) -> Self {
        let stamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
        Self {
            inner: tokio::sync::Mutex::new(SinkInner {
                file: JsonlFile::new(viewer_logs_dir.join(format!("{stamp}.jsonl"))),
                next_seq: 0,
            }),
        }
    }
}

/// The capture forwarder's sink: stamp, append to the logfile, then
/// broadcast as `logs://appended`. The emit happens even if the disk
/// write failed — the live view must not die with the disk.
#[tauri::command]
pub async fn logs_report(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, super::ShellModel>,
    sink: tauri::State<'_, LogSink>,
    level: String,
    message: String,
    detail: Option<String>,
) -> Result<(), String> {
    let label = webview.label().to_string();
    let source = match super::tab_id(&label) {
        Some(id) => model.tab_title(id).await.unwrap_or(label),
        // The chrome (strip + status bar) reports under one friendly
        // name — which window's chrome it was doesn't matter to the
        // reader.
        None if label.starts_with("chrome-") => "viewer-container".to_string(),
        None => label,
    };
    let entry = {
        let mut inner = sink.inner.lock().await;
        inner.next_seq += 1;
        let entry = LogEntry {
            seq: inner.next_seq,
            at_ms: super::now_ms(),
            source,
            level,
            message,
            detail,
        };
        inner.file.append(&entry).await;
        entry
    };
    let _ = app.emit("logs://appended", &entry);
    Ok(())
}

/// Stream this run's logfile BACKWARDS — newest entry first —
/// through `on_entry`, up to `count` entries (default and cap
/// [`PULL_DEFAULT`]). Only bytes fully written before the pull began
/// are read; anything newer reaches the consumer as `logs://appended`
/// instead (that is the whole disambiguation: pulls prepend history,
/// events append the present).
#[tauri::command]
pub async fn logs_pull(
    sink: tauri::State<'_, LogSink>,
    count: Option<u64>,
    on_entry: tauri::ipc::Channel<LogEntry>,
) -> Result<(), String> {
    let (path, len) = {
        let inner = sink.inner.lock().await;
        (inner.file.path().to_path_buf(), inner.file.written_len())
    };
    let max = count.unwrap_or(PULL_DEFAULT).min(PULL_DEFAULT);
    pull_backwards::<LogEntry, _>(&path, len, max, |entry| on_entry.send(entry).is_ok()).await
}
