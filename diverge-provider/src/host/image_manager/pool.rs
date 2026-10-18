//! A pool read: a manifest and a blob by digest, for the registry
//! and the OCI store alike.

use std::path::{Path, PathBuf};

use bytes::Bytes;
use serde_json::Value;
use tokio::fs;

use super::{Pieces, pieces};
use crate::host::image_registry::Digest;
use crate::protocol::image_source::Manifest;

/// The media type a manifest is served as where it does not name its
/// own: the OCI image manifest's.
const MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";

/// Where the blob under `digest` is in the pool at `pool`:
/// `blobs/<algorithm>/<hex>`, as an OCI layout keeps them. A parsed
/// digest is two names and nothing a path could not be made of.
pub fn blob_path(pool: &Path, digest: &Digest) -> PathBuf {
    pool.join("blobs").join(digest.algorithm()).join(digest.hex())
}

/// The size of the blob under `digest`, or `None` where the pool
/// holds no such file.
pub async fn blob_size(pool: &Path, digest: &Digest) -> Option<u64> {
    let found = fs::metadata(blob_path(pool, digest)).await.ok()?;
    found.is_file().then(|| found.len())
}

/// The manifest under `digest`: the blob read whole, found to be the
/// digest's and to be JSON, with the media type it names for itself,
/// or the OCI image manifest's where it names none. `None` for a
/// blob the pool does not hold, one that is not the digest's, and
/// one that is not JSON, which no manifest is.
pub async fn manifest(pool: &Path, digest: &Digest) -> Option<Manifest> {
    let body = fs::read(blob_path(pool, digest)).await.ok()?;
    if !digest.verifies(&body) {
        return None;
    }
    let value: Value = serde_json::from_slice(&body).ok()?;
    let media_type = value
        .get("mediaType")
        .and_then(Value::as_str)
        .unwrap_or(MANIFEST_MEDIA_TYPE)
        .to_string();
    Some(Manifest {
        media_type,
        body: Bytes::from(body),
    })
}

/// The blob under `digest` as pieces, or `None` for one the pool
/// does not hold.
pub async fn blob(pool: &Path, digest: &Digest) -> Option<Pieces> {
    pieces(&blob_path(pool, digest)).await
}
