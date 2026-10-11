//! The store: the root, the records, and the names.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// One image's layout, as `index.json` describes its one manifest:
/// the digest the record carries and a pull serves, the manifest's
/// media type, and its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The manifest digest, `<algorithm>:<hex>`.
    pub digest: String,
    /// The manifest's media type.
    pub media_type: String,
    /// The manifest's size in bytes.
    pub size: u64,
}

/// The client image store: where the root, the records and the
/// layouts are, how much it may hold, and the names, one caller apart
/// from another.
/// Shared by the manager, which builds, lists and removes; the
/// deployer, which runs a caller's own image from it; and the
/// checker, which answers the `client` source from it.
#[derive(Debug)]
pub struct Store {
    /// The podman storage root, `<path>/root`, as podman sees it.
    root: String,
    /// The records, `<path>/meta`, on this host.
    meta: PathBuf,
    /// The layouts, `<path>/layouts`, on this host: each image
    /// exported once, the bytes a pull serves.
    layouts: PathBuf,
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
            layouts: images.path.join("layouts"),
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

    /// Where one image's layout is: `<layouts>/<identity>/<name>/`.
    pub fn layout_dir(&self, client_identity: &str, name: &str) -> PathBuf {
        self.layouts.join(client_identity).join(name)
    }

    /// The layout at `dir` read: its `index.json`'s one manifest. A
    /// layout with no index, or an index that is not one manifest's
    /// digest, media type and size, is [`Error::Layout`].
    pub async fn read_layout(dir: &Path) -> Result<Layout, Error> {
        let index: Value = serde_json::from_slice(&fs::read(dir.join("index.json")).await?)?;
        let manifest = index
            .get("manifests")
            .and_then(|manifests| manifests.get(0))
            .ok_or_else(|| Error::Layout(dir.to_path_buf()))?;
        let text = |key: &str| manifest.get(key).and_then(Value::as_str).map(str::to_string);
        match (text("digest"), text("mediaType"), manifest.get("size").and_then(Value::as_u64)) {
            (Some(digest), Some(media_type), Some(size)) => Ok(Layout { digest, media_type, size }),
            _ => Err(Error::Layout(dir.to_path_buf())),
        }
    }

    /// Where a blob of the layout at `dir` is, by digest:
    /// `blobs/<algorithm>/<hex>`. `None` for a digest that is not
    /// two names joined by `:`, which a path could not be made of.
    pub fn blob_path(dir: &Path, digest: &str) -> Option<PathBuf> {
        let (algorithm, hex) = digest.split_once(':')?;
        let name = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric());
        (name(algorithm) && name(hex)).then(|| dir.join("blobs").join(algorithm).join(hex))
    }

    /// How many bytes the layout at `dir` holds: every blob's size.
    /// A layout with no blobs holds none.
    pub async fn layout_bytes(dir: &Path) -> Result<u64, Error> {
        let mut algorithms = match fs::read_dir(dir.join("blobs")).await {
            Ok(algorithms) => algorithms,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error.into()),
        };
        let mut total = 0;
        while let Some(algorithm) = algorithms.next_entry().await? {
            let mut blobs = fs::read_dir(algorithm.path()).await?;
            while let Some(blob) = blobs.next_entry().await? {
                total += blob.metadata().await?.len();
            }
        }
        Ok(total)
    }

    /// The layout removed; one already gone is removed.
    pub async fn remove_layout(&self, client_identity: &str, name: &str) -> Result<(), Error> {
        match fs::remove_dir_all(self.layout_dir(client_identity, name)).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
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

    /// How many bytes the store holds: the root as the cache is
    /// counted — every image listed, every layer, shared or not — and
    /// every layout's blobs.
    pub async fn used(&self) -> Result<u64, Error> {
        let images = podman::images_in(&self.root).await.map_err(Error::Podman)?;
        let mut total: u64 = images.iter().map(|image| image.size).sum();
        let mut identities = match fs::read_dir(&self.layouts).await {
            Ok(identities) => identities,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(total),
            Err(error) => return Err(error.into()),
        };
        while let Some(identity) = identities.next_entry().await? {
            let mut names = fs::read_dir(identity.path()).await?;
            while let Some(name) = names.next_entry().await? {
                total += Store::layout_bytes(&name.path()).await?;
            }
        }
        Ok(total)
    }
}

/// Whether `name` may name a client image: one segment of a
/// repository path, so that it is a segment of the tag and the stem
/// of the record's file.
pub fn name_ok_segment(name: &str) -> bool {
    name_ok(name) && !name.contains('/')
}
