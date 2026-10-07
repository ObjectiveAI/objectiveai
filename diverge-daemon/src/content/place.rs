//! Putting received content where its id lives.

use std::path::Path;

use sha2::{Digest as _, Sha256};

use super::Error;

/// The digest a hasher has, as bytes.
pub fn hash_of(hasher: Sha256) -> Vec<u8> {
    hasher.finalize().to_vec()
}

/// Where a held resource's bytes are: `<resources>/<id>`.
pub fn held(resources: &Path, id: &str) -> std::path::PathBuf {
    resources.join(id)
}

/// Move the received content at `incoming` to the id's place under
/// `resources`, or throw it away when the id is held already — the
/// bytes are the same bytes. `true` is the content placed anew.
pub async fn place(resources: &Path, incoming: &Path, id: &str) -> Result<bool, Error> {
    let at = held(resources, id);
    if tokio::fs::try_exists(&at).await.unwrap_or(false) {
        discard(incoming).await;
        return Ok(false);
    }
    tokio::fs::rename(incoming, &at).await.map_err(|source| Error::Io {
        path: at.clone(),
        source,
    })?;
    // A file resource arrived inside its own incoming directory,
    // which is now empty beside it and goes.
    if let Some(parent) = incoming.parent()
        && parent != resources.join("incoming")
    {
        let _ = tokio::fs::remove_dir(parent).await;
    }
    Ok(true)
}

/// Remove received content that will not be placed.
pub async fn discard(incoming: &Path) {
    if tokio::fs::metadata(incoming).await.is_ok_and(|meta| meta.is_dir()) {
        let _ = tokio::fs::remove_dir_all(incoming).await;
    } else {
        let _ = tokio::fs::remove_file(incoming).await;
        if let Some(parent) = incoming.parent() {
            let _ = tokio::fs::remove_dir(parent).await;
        }
    }
}
