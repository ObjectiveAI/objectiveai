use indexmap::IndexMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Maximum number of entries in the file state cache.
const MAX_CACHE_ENTRIES: usize = 100;

/// Cached state for a file that has been read.
#[derive(Debug, Clone)]
pub struct FileStateEntry {
    /// Normalized file content (CRLF→LF).
    pub content: String,
    /// File modification time in milliseconds since epoch.
    pub timestamp: u64,
    /// Line offset used when reading (None = full read).
    pub offset: Option<usize>,
    /// Line limit used when reading (None = full read).
    pub limit: Option<usize>,
    /// Whether this entry represents an injected/transformed partial view.
    /// Range reads with offset/limit are legitimate and do NOT set this flag.
    pub is_partial_view: bool,
}

impl FileStateEntry {
    /// Returns true if this entry represents a partial view that blocks
    /// writes/edits (injected or transformed content, not range reads).
    pub fn is_partial_view(&self) -> bool {
        self.is_partial_view
    }
}

/// Normalize a cache key path: canonicalize if possible, otherwise return as-is.
async fn normalize_cache_key(path: &str) -> String {
    match tokio::fs::canonicalize(Path::new(path)).await {
        Ok(canonical) => canonical.to_string_lossy().to_string(),
        Err(_) => path.to_string(),
    }
}

/// Per-session cache tracking files that have been read.
/// Write and edit operations require a file to be in this cache
/// (and not be a partial view) before they can proceed.
/// Limited to MAX_CACHE_ENTRIES entries; oldest entry is evicted when full.
#[derive(Debug, Clone)]
pub struct FileStateCache {
    inner: Arc<RwLock<IndexMap<String, FileStateEntry>>>,
}

impl FileStateCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(IndexMap::new())),
        }
    }

    /// Get the cached state for a file path.
    pub async fn get(&self, path: &str) -> Option<FileStateEntry> {
        let key = normalize_cache_key(path).await;
        self.inner.read().unwrap().get(&key).cloned()
    }

    /// Set the cached state for a file path.
    pub async fn set(&self, path: String, entry: FileStateEntry) {
        let key = normalize_cache_key(&path).await;
        let mut inner = self.inner.write().unwrap();
        // Evict an entry if we're at capacity and this is a new key
        if inner.len() >= MAX_CACHE_ENTRIES && !inner.contains_key(&key) {
            if let Some(first_key) = inner.keys().next().cloned() {
                inner.shift_remove(&first_key);
            }
        }
        inner.insert(key, entry);
    }

    /// Clear all cached state.
    pub fn clear(&self) {
        self.inner.write().unwrap().clear();
    }
}
