//! Taking a held resource's bytes away.

use std::path::Path;

use super::{Error, held};

/// Remove the bytes held under `id`: the file, or the whole tree. A
/// resource whose bytes are gone already is nothing to remove.
pub async fn remove(resources: &Path, id: &str) -> Result<(), Error> {
    let at = held(resources, id);
    let Ok(meta) = tokio::fs::metadata(&at).await else {
        return Ok(());
    };
    let removed = if meta.is_dir() {
        tokio::fs::remove_dir_all(&at).await
    } else {
        tokio::fs::remove_file(&at).await
    };
    removed.map_err(|source| Error::Io { path: at, source })
}
