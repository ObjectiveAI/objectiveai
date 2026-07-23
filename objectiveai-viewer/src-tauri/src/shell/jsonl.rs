//! The shared JSONL machinery both log sinks ride: an append-only
//! file with a torn-line-proof watermark, and a backwards (newest
//! first) streaming reader. Rust-side log storage is DISK, not
//! memory — sinks hold O(1) state, readers hold O(window).

use std::path::{Path, PathBuf};

use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

/// Backwards-read window size.
const CHUNK: u64 = 64 * 1024;

/// Epoch millis now — every log entry's Rust-stamped clock.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// One append-only JSONL file. Opens lazily on the first append
/// (creating parent directories), and every write is best-effort —
/// logging must never take the app down. A failed writer is dropped
/// and the next append retries the open.
pub struct JsonlFile {
    path: PathBuf,
    file: Option<tokio::fs::File>,
    /// Bytes FULLY written — a concurrent pull reads at most this
    /// far, so it can never observe a torn line.
    written_len: u64,
}

impl JsonlFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            file: None,
            written_len: 0,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn written_len(&self) -> u64 {
        self.written_len
    }

    /// Drop the OS handle (an ended stream shouldn't hold one open);
    /// the watermark survives, and a later append reopens.
    pub fn close_handle(&mut self) {
        self.file = None;
    }

    /// Append one value as a JSONL line.
    pub async fn append(&mut self, value: &impl serde::Serialize) {
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
        let Ok(mut line) = serde_json::to_vec(value) else {
            return;
        };
        line.push(b'\n');
        if file.write_all(&line).await.is_ok() {
            self.written_len += line.len() as u64;
        } else {
            self.file = None;
        }
    }
}

/// Stream a JSONL file BACKWARDS — newest line first — through
/// `send`, up to `max` entries, reading at most `len` bytes (the
/// caller's watermark). Windowed reads keep memory O([`CHUNK`]) no
/// matter the file size. A malformed line (torn by a crash mid-write)
/// is skipped, not fatal — this is a post-mortem surface. `send`
/// returning `false` (consumer gone) stops the stream.
pub async fn pull_backwards<T, F>(
    path: &Path,
    len: u64,
    max: u64,
    mut send: F,
) -> Result<(), String>
where
    T: serde::de::DeserializeOwned,
    F: FnMut(T) -> bool,
{
    if len == 0 || max == 0 {
        return Ok(());
    }
    let mut file = tokio::fs::File::open(path)
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
            if let Ok(entry) = serde_json::from_slice::<T>(line) {
                if !send(entry) {
                    return Ok(());
                }
                sent += 1;
            }
        }
        tail = carry;
        remaining = start;
    }
    Ok(())
}
