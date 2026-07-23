//! Cross-webview log capture. Every webview — chrome and content
//! alike — gets [`CAPTURE_INIT_SCRIPT`] as an initialization script:
//! it runs at document start, wraps every `console.*` method plus
//! uncaught errors and unhandled rejections, and forwards them to
//! [`logs_report`], buffering until the IPC bootstrap is up. Because
//! it needs NO cooperation from the page's own code, a webview whose
//! bundle fails to boot still reports its death.
//!
//! The store is RUST-SIDE by design: capture runs whether or not the
//! "viewer logs" tab exists, from the first webview's first byte, and
//! survives the tab being closed, reopened, or reparented — the tab
//! is a pure view (a `logs_snapshot` boot read + `logs://appended`
//! upserts, keyed by `seq`).

use std::collections::VecDeque;

use tauri::Emitter;

/// The capture forwarder, injected into every webview builder.
pub const CAPTURE_INIT_SCRIPT: &str = include_str!("capture.js");

/// Ring cap — the oldest entries fall off.
const CAP: usize = 1000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
    /// Monotonic, never reused — the view's upsert key.
    pub seq: u64,
    /// Epoch millis, stamped by RUST on receipt (a webview does not
    /// get to claim its own clock).
    pub at_ms: u64,
    /// Where it came from: a content webview reports as its tab's
    /// TITLE (resolved at receipt), the chrome as its own label.
    pub source: String,
    /// A console level (`log`/`info`/`warn`/`error`/`debug`/`trace`),
    /// or `uncaught` / `unhandledrejection`.
    pub level: String,
    pub message: String,
    /// Stack trace, when there is one.
    pub detail: Option<String>,
    /// Consecutive identical (source, level, message) reports
    /// coalesce into one entry with a bumped count — a tight error
    /// loop must not wash the whole ring.
    pub count: u64,
}

/// The managed ring store.
#[derive(Default)]
pub struct LogStore {
    inner: tokio::sync::Mutex<LogsInner>,
}

#[derive(Default)]
struct LogsInner {
    entries: VecDeque<LogEntry>,
    next_seq: u64,
}

/// The capture forwarder's sink. Appends (or coalesces into) the
/// ring, then broadcasts the touched entry as `logs://appended` —
/// the logs tab upserts by `seq`, so a coalesced bump is just a
/// re-broadcast of the same entry with a higher count.
#[tauri::command]
pub async fn logs_report(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, super::ShellModel>,
    store: tauri::State<'_, LogStore>,
    level: String,
    message: String,
    detail: Option<String>,
) -> Result<(), String> {
    let label = webview.label().to_string();
    let source = match super::tab_id(&label) {
        Some(id) => model.tab_title(id).await.unwrap_or(label),
        None => label,
    };
    let at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let entry = {
        let mut inner = store.inner.lock().await;
        let coalesce = inner.entries.back().is_some_and(|last| {
            last.source == source && last.level == level && last.message == message
        });
        if coalesce {
            let last = inner.entries.back_mut().expect("checked above");
            last.count += 1;
            if last.detail.is_none() {
                last.detail = detail;
            }
            last.clone()
        } else {
            inner.next_seq += 1;
            let entry = LogEntry {
                seq: inner.next_seq,
                at_ms,
                source,
                level,
                message,
                detail,
                count: 1,
            };
            inner.entries.push_back(entry.clone());
            while inner.entries.len() > CAP {
                inner.entries.pop_front();
            }
            entry
        }
    };
    let _ = app.emit("logs://appended", &entry);
    Ok(())
}

/// The whole ring, oldest first — the logs tab's boot read (subscribe
/// to `logs://appended` FIRST, then snapshot; upsert both by `seq`).
#[tauri::command]
pub async fn logs_snapshot(
    store: tauri::State<'_, LogStore>,
) -> Result<Vec<LogEntry>, String> {
    let inner = store.inner.lock().await;
    Ok(inner.entries.iter().cloned().collect())
}
