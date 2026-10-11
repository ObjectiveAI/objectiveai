//! The store: the root, the records, and the names.

use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::fs;
use tokio::sync::Mutex;

use super::Error;
use crate::host::container_deployer::{name_ok, tool_path};
use crate::host::tools::podman;
use diverge_sdk::config::provider::images;
use diverge_sdk::provider::endpoints::images::list::server::response::Image;

/// An image's record, `meta/<identity>/<name>.json`: what podman does
/// not keep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    /// The manifest digest podman answered for the image built.
    pub digest: String,
    /// Its size in BYTES, as podman counted it.
    pub bytes: u64,
    /// When the build ended, in seconds since the epoch.
    pub created: u64,
    /// The Containerfile, verbatim.
    pub containerfile: String,
}

/// The client image store: where the root and the records are, how
/// much it may hold, and the names, one caller apart from another.
/// Shared by the manager, which builds, lists and removes; the
/// deployer, which runs a caller's own image from it; and the
/// checker, which answers the `client` source from it.
#[derive(Debug)]
pub struct Store {
    /// The podman storage root, `<path>/root`, as podman sees it.
    root: String,
    /// The records, `<path>/meta`, on this host.
    meta: PathBuf,
    /// The most the root may hold, in BYTES.
    capacity: u64,
    /// One lock per identity, held across a build or a delete of a
    /// name, so two builds of one name do not race each other.
    namespaces: DashMap<String, Arc<Mutex<()>>>,
}

impl Store {
    /// The store the `images` section names.
    pub fn new(images: &images::Images) -> Self {
        Store {
            root: tool_path(&images.path.join("root")),
            meta: images.path.join("meta"),
            capacity: images.capacity,
            namespaces: DashMap::new(),
        }
    }

    /// The podman storage root, as podman is handed it.
    pub fn root(&self) -> &str {
        &self.root
    }

    /// The most the root may hold, in BYTES.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// The name an image is built under in the root, and copied into
    /// the cache under: `localhost/diverge/<identity>/<name>`, the
    /// identity as sixteen hexadecimal characters of its SHA-256, so
    /// that it is a repository path whatever the identity is.
    pub fn tag(&self, client_identity: &str, name: &str) -> String {
        let hashed = hex::encode(&Sha256::digest(client_identity.as_bytes())[..8]);
        format!("{}{hashed}/{name}", podman::CLIENT_PREFIX)
    }

    /// The identity's lock, for a build or a delete of one of its
    /// names.
    pub fn namespace(&self, client_identity: &str) -> Arc<Mutex<()>> {
        Arc::clone(
            &self
                .namespaces
                .entry(client_identity.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    }

    /// Where the identity's records are.
    fn meta_dir(&self, client_identity: &str) -> PathBuf {
        self.meta.join(client_identity)
    }

    /// Where one record is.
    fn meta_path(&self, client_identity: &str, name: &str) -> PathBuf {
        self.meta_dir(client_identity).join(format!("{name}.json"))
    }

    /// The record under `name` for the identity, or `None` where
    /// there is none.
    pub async fn read(&self, client_identity: &str, name: &str) -> Result<Option<Meta>, Error> {
        match fs::read(self.meta_path(client_identity, name)).await {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// The record written, whole or not at all: to a sibling, then
    /// renamed over.
    pub async fn write(&self, client_identity: &str, name: &str, meta: &Meta) -> Result<(), Error> {
        fs::create_dir_all(self.meta_dir(client_identity)).await?;
        let path = self.meta_path(client_identity, name);
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, serde_json::to_vec(meta)?).await?;
        fs::rename(&temporary, &path).await?;
        Ok(())
    }

    /// The record removed; one already gone is removed.
    pub async fn remove(&self, client_identity: &str, name: &str) -> Result<(), Error> {
        match fs::remove_file(self.meta_path(client_identity, name)).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Every image the identity has, as a listing reports them: the
    /// identity's records read, in the directory's order. An identity
    /// with no directory has none. A record that will not parse is
    /// the error.
    pub async fn list(&self, client_identity: &str) -> Result<Vec<Image>, Error> {
        let mut entries = match fs::read_dir(self.meta_dir(client_identity)).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut images = Vec::new();
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
            images.push(Image {
                name: name.to_string(),
                digest: meta.digest,
                bytes: meta.bytes,
                created: meta.created,
            });
        }
        Ok(images)
    }

    /// The identity's image under `digest`, if it has one: its name
    /// and its record. A store that cannot be read has none.
    pub async fn find_digest(&self, client_identity: &str, digest: &str) -> Option<(String, Meta)> {
        let listed = self.list(client_identity).await.ok()?;
        let image = listed.into_iter().find(|image| image.digest == digest)?;
        let meta = self.read(client_identity, &image.name).await.ok()??;
        Some((image.name, meta))
    }

    /// How many bytes the root holds, counted as the cache is
    /// counted: every image listed, every layer, shared or not.
    pub async fn used(&self) -> Result<u64, Error> {
        let images = podman::images_in(&self.root).await.map_err(Error::Podman)?;
        Ok(images.iter().map(|image| image.size).sum())
    }
}

/// Whether `name` may name a client image: one segment of a
/// repository path, so that it is a segment of the tag and the stem
/// of the record's file.
pub fn name_ok_segment(name: &str) -> bool {
    name_ok(name) && !name.contains('/')
}
