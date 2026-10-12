//! A pool answered: one identity's client images, from the store's
//! disk.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::response::Response;
use tokio::fs;

use super::{Digest, blob_found, manifest_found, not_found, verified};
use crate::host::image_manager::{Store, pool};

/// The pool the repository named `repository` is, if it is one: the
/// store has to be there, the name has to be a pool's as the store
/// names them, and the directory has to exist. Anything else is no
/// repository. The name is a hash of the identity, so nothing of an
/// identity is in the path, and a pull that knows the hash and the
/// digest is one the identity itself started.
pub async fn stored(store: &Option<Arc<Store>>, repository: &str) -> Option<PathBuf> {
    let store = store.as_ref()?;
    if !Store::hash_ok(repository) {
        return None;
    }
    let pool = store.pool_dir_of(repository);
    fs::metadata(&pool).await.ok()?.is_dir().then_some(pool)
}

/// A manifest by reference, from the pool: the reference must be a
/// digest, as for a run's repository; the manifest is the pool's
/// blob under it, read whole.
pub async fn manifest(pool: &Path, reference: &str, head: bool) -> Response {
    let Some(digest) = Digest::parse(reference) else {
        return not_found("MANIFEST_UNKNOWN", "manifests are served by digest");
    };
    let Some(found) = pool::manifest(pool, &digest).await else {
        return not_found("MANIFEST_UNKNOWN", "the pool does not hold the manifest");
    };
    manifest_found(&digest, found, head)
}

/// A blob by digest, from the pool: a blob the pool does not hold is
/// `404` to a `HEAD` as to a `GET`, since the disk can say, and its
/// length is always stated; a `GET` is the file's pieces streamed
/// through [`verified`], so a file that is not its digest's is never
/// completed.
pub async fn blob(pool: &Path, digest: &str, head: bool) -> Response {
    let Some(digest) = Digest::parse(digest) else {
        return not_found("BLOB_UNKNOWN", "not a digest");
    };
    let Some(size) = pool::blob_size(pool, &digest).await else {
        return not_found("BLOB_UNKNOWN", "the pool does not hold the blob");
    };
    let body = if head {
        Body::empty()
    } else {
        let Some(pieces) = pool::blob(pool, &digest).await else {
            return not_found("BLOB_UNKNOWN", "the pool does not hold the blob");
        };
        Body::from_stream(verified(digest.clone(), pieces))
    };
    blob_found(&digest, Some(size), body)
}
