//! Throwing a directory away without waiting for it to be gone.

use std::io;
use std::path::Path;

/// Throw away whatever is at `dir`, and every `<dir>.trash-*` an
/// earlier start left beside it.
///
/// Renamed to `<dir>.trash-<pid>` first, so that the name is free at
/// once, then deleted best-effort: on Windows a directory being
/// deleted is still in the way of one made in its place until the
/// last handle on it closes, and a rename is not. Only the rename can
/// fail this; a deletion that does not finish is tried again by the
/// next start. A `dir` that is absent is nothing to do.
pub async fn discard(dir: &Path) -> io::Result<()> {
    let (Some(parent), Some(name)) = (dir.parent(), dir.file_name()) else {
        return Ok(());
    };
    let prefix = format!("{}.trash-", name.to_string_lossy());
    if let Ok(mut entries) = tokio::fs::read_dir(parent).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                let _ = tokio::fs::remove_dir_all(entry.path()).await;
            }
        }
    }
    if tokio::fs::try_exists(dir).await.unwrap_or(false) {
        let trash = parent.join(format!("{prefix}{}", std::process::id()));
        tokio::fs::rename(dir, &trash).await?;
        let _ = tokio::fs::remove_dir_all(&trash).await;
    }
    Ok(())
}
