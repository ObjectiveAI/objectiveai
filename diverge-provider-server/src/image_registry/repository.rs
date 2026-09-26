//! One run's source, and what it has answered.

use dashmap::DashMap;
use diverge_sdk::provider::server::image_source::{BlobStream, ImageSource, Manifest};
use serde_json::Value;

use super::Digest;

/// A repository being served: the run's [`ImageSource`], the
/// manifests it has answered, and the sizes their descriptors
/// declare.
///
/// A manifest is kept once fetched and verified: podman asks for one
/// more than once — a `HEAD` and a `GET`, an index and then its
/// platform's manifest — and a manifest is small. Its descriptors
/// name the config, the layers, and an index's manifests with their
/// sizes, and those are kept too, so a blob's `Content-Length` can be
/// stated when it is known and a `HEAD` on a blob asks the caller
/// nothing. A blob is never kept: it streams through.
#[derive(Debug)]
pub struct Repository {
    source: ImageSource,
    manifests: DashMap<String, Manifest>,
    sizes: DashMap<String, u64>,
}

impl Repository {
    pub fn new(source: ImageSource) -> Self {
        Repository {
            source,
            manifests: DashMap::new(),
            sizes: DashMap::new(),
        }
    }

    /// The manifest under `digest`: the one already answered, else the
    /// caller's, verified against the digest and kept. A manifest the
    /// caller does not hold, and one whose bytes are not the
    /// digest's, are `None`.
    pub async fn manifest(&self, digest: &Digest) -> Option<Manifest> {
        let key = digest.to_string();
        if let Some(manifest) = self.manifests.get(&key) {
            return Some(manifest.clone());
        }
        let manifest = self.source.manifest(&key).await?;
        if !digest.verifies(&manifest.body) {
            return None;
        }
        self.remember_sizes(&manifest.body);
        self.manifests.insert(key, manifest.clone());
        Some(manifest)
    }

    /// The blob under `digest`, as the caller's pieces, or `None` for
    /// one the caller does not hold. Nothing is kept.
    pub async fn blob(&self, digest: &Digest) -> Option<BlobStream> {
        self.source.blob(&digest.to_string()).await
    }

    /// The size a manifest's descriptor declared for `digest`, if one
    /// has.
    pub fn size(&self, digest: &Digest) -> Option<u64> {
        self.sizes.get(&digest.to_string()).map(|size| *size)
    }

    /// Every descriptor in the manifest — `config`, each of `layers`,
    /// each of an index's `manifests` — entered by its digest with
    /// its size. A manifest that is not JSON, or a descriptor missing
    /// either field, contributes nothing; the manifest is served as
    /// it is regardless, since its bytes are the digest's.
    fn remember_sizes(&self, body: &[u8]) {
        let Ok(manifest) = serde_json::from_slice::<Value>(body) else {
            return;
        };
        let descriptors = manifest
            .get("config")
            .into_iter()
            .chain(manifest.get("layers").and_then(Value::as_array).into_iter().flatten())
            .chain(manifest.get("manifests").and_then(Value::as_array).into_iter().flatten());
        for descriptor in descriptors {
            let (Some(digest), Some(size)) = (
                descriptor.get("digest").and_then(Value::as_str),
                descriptor.get("size").and_then(Value::as_u64),
            ) else {
                continue;
            };
            self.sizes.insert(digest.to_string(), size);
        }
    }
}
