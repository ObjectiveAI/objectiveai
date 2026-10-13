//! Writing a document.

use std::path::Path;

use super::{Error, Which};

/// Put `yaml` in the document's file at `root`, whole and at once:
/// written beside it under the file's name with `.tmp` after, then
/// renamed over it, so a reader finds the old document or the new and
/// never the half between.
pub async fn write(root: &Path, which: Which, yaml: &str) -> Result<(), Error> {
    let file = root.join(which.file());
    let temp = root.join(format!("{}.tmp", which.file()));
    tokio::fs::write(&temp, yaml).await.map_err(|source| Error::Io {
        path: temp.clone(),
        source,
    })?;
    tokio::fs::rename(&temp, &file).await.map_err(|source| Error::Io { path: file, source })
}
