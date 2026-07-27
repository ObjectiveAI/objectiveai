//! Chromium's own diagnostics, forwarded into the viewer's log inbox.
//!
//! Every other surface in the viewer reports into `viewer-logs`: a
//! content webview through [`crate::shell::CAPTURE_INIT_SCRIPT`], the
//! shell through `report_shell`. CEF has neither — it takes a
//! `log_file` path in its settings and writes there, from EVERY
//! Chromium process at once, and offers the embedder no callback at
//! all. So the only way to see what Chromium is saying is to tail that
//! file.
//!
//! Which is worth doing, because those lines are the ONLY account of
//! whole classes of failure. A GPU process that crashes on startup, a
//! profile path Chromium refused, a renderer that died — all of them
//! present to the user as a blank browser tab and to the embedder as
//! silence.
//!
//! Two deliberate choices:
//!
//! - **From this run's offset only.** The file is append-only across
//!   viewer runs, so the pump starts at whatever length it had before
//!   `initialize` — the log tab shows this session, not a replay of
//!   every session before it.
//! - **Consecutive duplicates collapse.** Chromium has heartbeat
//!   diagnostics that repeat verbatim every few seconds forever (the
//!   Hyper-V CPU-probe counter is one); left alone they would bury
//!   every real entry.

use std::path::{Path, PathBuf};

use tokio::io::{AsyncBufReadExt as _, AsyncSeekExt as _};

/// How often the tail is checked. A poll, because a logfile written by
/// several processes has nothing to subscribe to — and because the
/// alternative (a filesystem watcher) would fire on the same schedule
/// anyway for a file appended to this often.
const POLL: std::time::Duration = std::time::Duration::from_millis(750);

/// Where CEF writes, given the profiles root it was initialized with.
pub fn log_file(root_cache: &Path) -> PathBuf {
    root_cache.join("cef-debug.log")
}

/// The file's current length — the offset the pump should start from.
/// Sampled BEFORE `cef::initialize`, so nothing a previous viewer run
/// wrote is replayed.
pub fn current_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Tail `cef-debug.log` into the viewer's log inbox, forever (the pump
/// dies with the process, like CEF itself).
pub fn spawn_pump(app: tauri::AppHandle, path: PathBuf, from: u64) {
    tauri::async_runtime::spawn(async move {
        let mut offset = from;
        let mut last: Option<String> = None;
        loop {
            tokio::time::sleep(POLL).await;
            let Ok(mut file) = tokio::fs::File::open(&path).await else {
                continue;
            };
            // A log that SHRANK was rotated or replaced under us; the
            // only sane read is to start over from its new beginning.
            let len = file.metadata().await.map(|m| m.len()).unwrap_or(0);
            if len < offset {
                offset = 0;
            }
            if len == offset {
                continue;
            }
            if file.seek(std::io::SeekFrom::Start(offset)).await.is_err() {
                continue;
            }
            let mut lines = tokio::io::BufReader::new(&mut file).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                offset += line.len() as u64 + 1;
                let line = line.trim().to_string();
                if line.is_empty() || last.as_deref() == Some(line.as_str()) {
                    continue;
                }
                let level = level_of(&line);
                last = Some(line.clone());
                crate::shell::report_as(&app, "cef".to_string(), level, line, None)
                    .await;
            }
            // Trust the metadata length over the byte count, which a
            // partially-written final line would leave short.
            offset = offset.min(len);
        }
    });
}

/// Chromium's severity, from its line prefix
/// (`[pid:tid:MMDD/HHMMSS.mmm:LEVEL:file.cc(line)] message`). Anything
/// unrecognized reads as `info` — a line we cannot classify is still a
/// line worth showing.
fn level_of(line: &str) -> &'static str {
    if line.contains(":ERROR:") || line.contains(":FATAL:") {
        "error"
    } else if line.contains(":WARNING:") {
        "warn"
    } else {
        "info"
    }
}
