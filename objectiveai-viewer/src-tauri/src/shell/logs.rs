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
//! newest first, straight off disk through an IPC channel, never
//! whole-file in memory. Newest-first is what disambiguates the two
//! flows at the consumer: historic pulls PREPEND (ever older), live
//! events APPEND (ever newer), and they interleave safely because
//! everything keys by `seq`. The JS side owns the ring/cap; Rust
//! holds O(1) state and the file survives a viewer crash for
//! post-mortem reading.

use std::path::PathBuf;

use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

/// The capture forwarder, injected into every webview builder.
pub const CAPTURE_INIT_SCRIPT: &str = include_str!("capture.js");

/// Default (and maximum) entries a [`logs_pull`] streams.
const PULL_DEFAULT: u64 = 1000;

/// Backwards-read window size.
const CHUNK: u64 = 64 * 1024;

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

/// The managed append-only sink. The file opens lazily on the first
/// report (and capture must never take the app down, so every write
/// is best-effort).
pub struct LogSink {
    inner: tokio::sync::Mutex<SinkInner>,
}

struct SinkInner {
    path: PathBuf,
    file: Option<tokio::fs::File>,
    /// Bytes FULLY written — a concurrent pull reads at most this
    /// far, so it can never observe a torn line.
    written_len: u64,
    next_seq: u64,
}

impl LogSink {
    /// A sink for this viewer RUN — the path carries the start
    /// timestamp, so every run appends to its own fresh file and old
    /// runs linger on disk for post-mortem reading.
    pub fn new(viewer_logs_dir: PathBuf) -> Self {
        let stamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
        Self {
            inner: tokio::sync::Mutex::new(SinkInner {
                path: viewer_logs_dir.join(format!("{stamp}.jsonl")),
                file: None,
                written_len: 0,
                next_seq: 0,
            }),
        }
    }
}

impl SinkInner {
    async fn append(&mut self, entry: &LogEntry) {
        if self.file.is_none() {
            if let Some(dir) = self.path.parent() {
                let _ = tokio::fs::create_dir_all(dir).await;
            }
            self.file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .await
                .ok();
        }
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let Ok(mut line) = serde_json::to_vec(entry) else {
            return;
        };
        line.push(b'\n');
        if file.write_all(&line).await.is_ok() {
            self.written_len += line.len() as u64;
        } else {
            // A failed writer never recovers mid-run — drop it and
            // let the next report retry the open.
            self.file = None;
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
    let at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let entry = {
        let mut inner = sink.inner.lock().await;
        inner.next_seq += 1;
        let entry = LogEntry {
            seq: inner.next_seq,
            at_ms,
            source,
            level,
            message,
            detail,
        };
        inner.append(&entry).await;
        entry
    };
    let _ = app.emit("logs://appended", &entry);
    Ok(())
}

/// Stream this run's logfile BACKWARDS — newest entry first —
/// through `on_entry`, up to `count` entries (default and cap
/// [`PULL_DEFAULT`]). Reads the file in windows from the end, so
/// memory stays O(window) no matter how large the file grew. Only
/// bytes fully written before the pull began are read; anything
/// newer reaches the consumer as `logs://appended` instead (that is
/// the whole disambiguation: pulls prepend history, events append
/// the present).
#[tauri::command]
pub async fn logs_pull(
    sink: tauri::State<'_, LogSink>,
    count: Option<u64>,
    on_entry: tauri::ipc::Channel<LogEntry>,
) -> Result<(), String> {
    let (path, len) = {
        let inner = sink.inner.lock().await;
        (inner.path.clone(), inner.written_len)
    };
    if len == 0 {
        return Ok(());
    }
    let max = count.unwrap_or(PULL_DEFAULT).min(PULL_DEFAULT);
    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|e| format!("logs: open {}: {e}", path.display()))?;
    let mut remaining = len;
    // The partial first piece of each window — a line whose start
    // lives in an earlier (not yet read) window.
    let mut tail: Vec<u8> = Vec::new();
    let mut sent = 0u64;
    while remaining > 0 && sent < max {
        let window = CHUNK.min(remaining);
        let start = remaining - window;
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; window as usize];
        file.read_exact(&mut buf)
            .await
            .map_err(|e| e.to_string())?;
        buf.extend_from_slice(&tail);
        let mut lines: Vec<&[u8]> = buf.split(|&b| b == b'\n').collect();
        // The first piece may continue into the previous window —
        // carry it, unless this window IS the file start.
        let carry = if start > 0 {
            lines.remove(0).to_vec()
        } else {
            Vec::new()
        };
        for line in lines.iter().rev() {
            if sent >= max {
                break;
            }
            if line.is_empty() {
                continue;
            }
            // A malformed line (torn by a crash mid-write) is
            // skipped, not fatal — this is a post-mortem surface.
            if let Ok(entry) = serde_json::from_slice::<LogEntry>(line) {
                let _ = on_entry.send(entry);
                sent += 1;
            }
        }
        tail = carry;
        remaining = start;
    }
    Ok(())
}
