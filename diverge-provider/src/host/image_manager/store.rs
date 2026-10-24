//! The store: the pools, the records, and the names.

use diverge_sdk::shared::Identity;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::fs;
use tokio::sync::Mutex;

use super::Error;
use crate::host::container_deployer::name_ok;
use crate::host::tools::podman;
use crate::config::images;
use diverge_sdk::provider::endpoints::images::list::server::response::Image;

/// An image's record, `<pool>/images/<name>.json`: what the pool's
/// blobs do not say of themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    /// The manifest digest of the image as it was exported, which a
    /// pull names it by.
    pub digest: String,
    /// Its size in BYTES: every blob of its export, shared with
    /// another of the identity's images or not.
    pub bytes: u64,
    /// When the build ended, in seconds since the epoch.
    pub created: u64,
    /// The Containerfile, verbatim.
    pub containerfile: String,
    /// Every blob of its export, `<algorithm>:<hex>`, the manifest
    /// among them: what a prune keeps for it.
    pub blobs: Vec<String>,
}

/// The client image store: where the pools are, how much they may
/// hold between them, and the names, one caller apart from another.
/// Shared by the manager, which builds, lists and removes; the
/// registry, which serves a pool to a pull; the deployer, which
/// pulls a caller's own image from it; the OCI store, which serves a
/// pool to the caller; and the checker, which answers the `client`
/// source from it.
#[derive(Debug)]
pub struct Store {
    /// The directory the pools are under, `<path>`, on this host.
    images: PathBuf,
    /// The most every pool may hold together, in BYTES.
    capacity: u64,
    /// One lock per identity, held across a build or a delete of a
    /// name, so two builds of one name do not race each other and a
    /// prune sees no blob still on its way into a record.
    namespaces: DashMap<String, Arc<Mutex<()>>>,
}

impl Store {
    /// The store the `images` section names.
    pub fn new(images: &images::Images) -> Self {
        Store {
            images: images.path.clone(),
            capacity: images.capacity,
            namespaces: DashMap::new(),
        }
    }

    /// The most every pool may hold together, in BYTES.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// The identity as its pool is named: sixteen hexadecimal
    /// characters of the SHA-256 of its issuer, a NUL and its subject,
    /// so that it is one segment of a
    /// repository path and one directory's name whatever the identity
    /// is, and so that the identity itself is written nowhere a pull
    /// could read it.
    pub fn hash(client_identity: &Identity) -> String {
        let pair = format!("{}\0{}", client_identity.issuer, client_identity.subject);
        hex::encode(&Sha256::digest(pair.as_bytes())[..8])
    }

    /// Whether `text` is a pool's name as [`hash`](Self::hash) makes
    /// one: exactly sixteen lowercase hexadecimal characters.
    pub fn hash_ok(text: &str) -> bool {
        text.len() == 16 && text.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    }

    /// Where the identity's pool is: `<images>/<hash>/`.
    pub fn pool_dir(&self, client_identity: &Identity) -> PathBuf {
        self.pool_dir_of(&Store::hash(client_identity))
    }

    /// Where the pool named `hash` is, `<images>/<hash>/`, whether or
    /// not there is one.
    pub fn pool_dir_of(&self, hash: &str) -> PathBuf {
        self.images.join(hash)
    }

    /// The name an image is pulled by from the provider's own
    /// registry at `port`, and built and held under in the cache:
    /// `127.0.0.1:<port>/<hash>/<name>`, the registry's repository
    /// being the pool. Every name under [`CLIENT_PREFIX`](podman::CLIENT_PREFIX)
    /// is one the cache's lookup by digest passes over.
    pub fn reference(&self, port: u16, client_identity: &Identity, name: &str) -> String {
        format!("{}{port}/{}/{name}", podman::CLIENT_PREFIX, Store::hash(client_identity))
    }

    /// The identity's lock, for a build or a delete of one of its
    /// names.
    pub fn namespace(&self, client_identity: &Identity) -> Arc<Mutex<()>> {
        Arc::clone(
            &self
                .namespaces
                .entry(client_identity.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    }

    /// Where the identity's records are, `<pool>/images/`.
    fn records_dir(&self, client_identity: &Identity) -> PathBuf {
        self.pool_dir(client_identity).join("images")
    }

    /// Where one record is.
    fn record_path(&self, client_identity: &Identity, name: &str) -> PathBuf {
        self.records_dir(client_identity).join(format!("{name}.json"))
    }

    /// The record under `name` for the identity, or `None` where
    /// there is none.
    pub async fn read(&self, client_identity: &Identity, name: &str) -> Result<Option<Meta>, Error> {
        match fs::read(self.record_path(client_identity, name)).await {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// The record written, whole or not at all: to a sibling, then
    /// renamed over.
    pub async fn write(&self, client_identity: &Identity, name: &str, meta: &Meta) -> Result<(), Error> {
        fs::create_dir_all(self.records_dir(client_identity)).await?;
        let path = self.record_path(client_identity, name);
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, serde_json::to_vec(meta)?).await?;
        fs::rename(&temporary, &path).await?;
        Ok(())
    }

    /// The record removed; one already gone is removed.
    pub async fn remove(&self, client_identity: &Identity, name: &str) -> Result<(), Error> {
        match fs::remove_file(self.record_path(client_identity, name)).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Every record the identity has, by name, in the directory's
    /// order. An identity with no directory has none. A record that
    /// will not parse is the error.
    async fn records(&self, client_identity: &Identity) -> Result<Vec<(String, Meta)>, Error> {
        let mut entries = match fs::read_dir(self.records_dir(client_identity)).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut records = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let Ok(file) = entry.file_name().into_string() else {
                continue;
            };
            let Some(name) = file.strip_suffix(".json") else {
                continue;
            };
            if !name_ok_segment(name) {
                continue;
            }
            let meta: Meta = serde_json::from_slice(&fs::read(entry.path()).await?)?;
            records.push((name.to_string(), meta));
        }
        Ok(records)
    }

    /// Every image the identity has, as a listing reports them.
    pub async fn list(&self, client_identity: &Identity) -> Result<Vec<Image>, Error> {
        Ok(self
            .records(client_identity)
            .await?
            .into_iter()
            .map(|(name, meta)| Image {
                name,
                digest: meta.digest,
                bytes: meta.bytes,
                created: meta.created,
            })
            .collect())
    }

    /// The identity's image under `digest`, if it has one: its name
    /// and its record. A store that cannot be read has none.
    pub async fn find_digest(&self, client_identity: &Identity, digest: &str) -> Option<(String, Meta)> {
        self.records(client_identity)
            .await
            .ok()?
            .into_iter()
            .find(|(_, meta)| meta.digest == digest)
    }

    /// Every blob of the identity's pool that no record of the
    /// identity names, removed. What a delete does after its record
    /// is gone and what a build undone does: the pool holds what the
    /// records name and nothing else. Under the identity's lock, so
    /// no build of the identity has a blob in the pool that its
    /// record does not name yet.
    pub async fn prune(&self, client_identity: &Identity) -> Result<(), Error> {
        let named: HashSet<String> = self
            .records(client_identity)
            .await?
            .into_iter()
            .flat_map(|(_, meta)| meta.blobs)
            .collect();
        let mut algorithms = match fs::read_dir(self.pool_dir(client_identity).join("blobs")).await {
            Ok(algorithms) => algorithms,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        while let Some(algorithm) = algorithms.next_entry().await? {
            let Ok(algorithm_name) = algorithm.file_name().into_string() else {
                continue;
            };
            let mut blobs = fs::read_dir(algorithm.path()).await?;
            while let Some(blob) = blobs.next_entry().await? {
                let Ok(hex) = blob.file_name().into_string() else {
                    continue;
                };
                if !named.contains(&format!("{algorithm_name}:{hex}")) {
                    fs::remove_file(blob.path()).await?;
                }
            }
        }
        Ok(())
    }

    /// How many bytes a build could still add, as of now: the
    /// capacity less what every pool holds together, never below
    /// `0`. A fact about an instant, measured by a walk of the pools.
    pub async fn room(&self) -> Result<u64, Error> {
        Ok(self.capacity.saturating_sub(self.used().await?))
    }

    /// How many bytes every pool holds together: every blob under
    /// `<images>/*/blobs/`, each counted once however many records
    /// name it. What the capacity is measured against.
    pub async fn used(&self) -> Result<u64, Error> {
        let mut total = 0;
        let mut pools = fs::read_dir(&self.images).await?;
        while let Some(pool) = pools.next_entry().await? {
            let mut algorithms = match fs::read_dir(pool.path().join("blobs")).await {
                Ok(algorithms) => algorithms,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            while let Some(algorithm) = algorithms.next_entry().await? {
                let mut blobs = fs::read_dir(algorithm.path()).await?;
                while let Some(blob) = blobs.next_entry().await? {
                    total += blob.metadata().await?.len();
                }
            }
        }
        Ok(total)
    }
}

/// Whether `name` may name a client image: one segment of a
/// repository path, so that it is a segment of the reference and the
/// stem of the record's file.
pub fn name_ok_segment(name: &str) -> bool {
    name_ok(name) && !name.contains('/')
}
