//! An image as the provider serves it to a caller: the one resolution
//! an `oci` scope opens with, and the image — its parts and its proof —
//! the scope serves.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use bytes::Bytes;

use super::manifest::{Descriptor, Parsed};
use super::fetch::{Repository, fill, plan_at};
use super::{Attempt, Error, Pieces, Store};
use crate::host::container_deployer::name_ok;
use crate::host::image_registry::Digest;
use crate::host::registry_client::RegistryClient;
use crate::protocol::image_source::Manifest;
use crate::protocol::oci_store;
use diverge_sdk::shared::containers::request::Image;

/// Where an image is served to a caller from, as [`resolve`] found it.
#[derive(Debug, Clone)]
pub enum Resolved {
    /// The caller's own Store: it owns the digest.
    Store(Digest),
    /// One repository of a registry the provider uses, which has the
    /// manifest: `manifest` its bytes, found to be the digest's.
    Registry {
        host: String,
        name: String,
        digest: Digest,
        manifest: Bytes,
    },
}

/// Where the image is served to this caller from, if anywhere.
///
/// The rules, in order:
///
/// 1. **A digest.** One that does not parse is served from nowhere.
/// 2. **The caller's Store.** A digest the caller owns in `store` is
///    the Store's. Only the caller's own records are read: another
///    identity's Store is never consulted, however alike the digests.
/// 3. **A registry.** Else the image's references, in the order it
///    lists them: each whose registry the configuration lists and
///    whose name is a repository path is asked for the manifest under
///    `<name>@<digest>`, through the same client a push fetches with,
///    and the first that answers with bytes that are the digest's is
///    the registry's. A reference naming a registry not listed, or a
///    name that is not a repository path, is never asked.
/// 4. **Nowhere.** Else not served — but a registry that could not be
///    asked, or that answered bytes that are not the digest's, with no
///    reference serving the image, is the failure to answer and not a
///    no.
///
/// What the provider holds on its own account — the cache of images a
/// run took from a caller, podman's copies — is never asked: no image
/// brought in for one caller is revealed to, or served to, another.
pub async fn resolve(store: Option<&Store>, registries: &RegistryClient, client_identity: &Identity, image: &Image) -> Result<Option<Resolved>, Error> {
    let Some(digest) = Digest::parse(&image.digest) else {
        return Ok(None);
    };
    if let Some(store) = store {
        if store.owned(client_identity, &digest).await?.is_some() {
            return Ok(Some(Resolved::Store(digest)));
        }
    }
    let mut failed = None;
    for reference in &image.references {
        if !registries.lists(&reference.registry) || !name_ok(&reference.name) {
            continue;
        }
        match registries.manifest(&reference.registry, &reference.name, &digest.to_string()).await {
            Ok(Some(manifest)) if digest.verifies(&manifest) => {
                return Ok(Some(Resolved::Registry {
                    host: reference.registry.clone(),
                    name: reference.name.clone(),
                    digest,
                    manifest,
                }));
            }
            Ok(Some(_)) => failed = Some(format!("`{}` answered a manifest that is not {digest}", reference.registry)),
            Ok(None) => {}
            Err(error) => failed = Some(error),
        }
    }
    match failed {
        Some(error) => Err(Error::Registry(error)),
        None => Ok(None),
    }
}

/// The image one `images::oci` scope serves.
#[derive(Debug)]
pub enum Served {
    /// From the caller's own Store.
    Store(Owned),
    /// From a registry, fetched through the blob store.
    Registry(Through),
}

/// An image served from the caller's own Store, as the Store holds it.
#[derive(Debug)]
pub struct Owned {
    /// The Store.
    pub(super) store: Arc<Store>,
    /// The caller, whose record of the image is read at every ask.
    pub(super) client_identity: Identity,
    /// The image's manifest digest.
    pub(super) digest: Digest,
}

/// An image served from a registry, through the blob store, under one
/// attempt held for the scope's life.
///
/// The rules:
///
/// 1. **Opened.** The attempt is entered and the manifest the
///    resolution answered written and pinned, before any ask.
/// 2. **A part present.** A manifest or blob the blob store holds is
///    pinned and served from it, and the registry is not asked.
/// 3. **A part absent.** Otherwise it is fetched from the registry,
///    verified against its digest, written and pinned, then served
///    from the blob store; one the registry does not supply, or
///    supplies bytes that are not its digest's, is the empty finish.
/// 4. **Proven.** A proof brings in the image's plan at the platform
///    asked for — see [`plan_at`] — every blob the blob store lacks
///    fetched from the registry, verified, written and pinned under the
///    same attempt; then [`Store::prove`] over the blob store's bytes.
///    A platform that breaks the rule, a part the registry does not
///    supply or supplies wrong, and a store that fails are no proof.
/// 5. **Closed.** At the scope's end, an image every part of which is
///    present — every part [`Store::reach`] names, and for an index at
///    least one platform manifest — is written a cache record, unless
///    an identity owns it, and the cache trimmed; then the attempt
///    leaves and the blob store is pruned, so what the scope brought
///    in and no record reaches goes.
///
/// No room is made before a part is written: the pins keep the scope's
/// parts past the capacity for its life, and the trim at its end
/// brings the store back under it, as a run's pull from a caller does.
#[derive(Debug)]
pub struct Through {
    /// The blob store: the Store, or the deployer's own with no room.
    pub(super) blobs: Arc<Store>,
    /// The client of the registries the configuration lists.
    pub(super) registries: Arc<RegistryClient>,
    /// The registry's host.
    pub(super) host: String,
    /// The repository's name in it.
    pub(super) name: String,
    /// The image's manifest digest.
    pub(super) digest: Digest,
    /// The attempt every part is pinned to.
    pub(super) attempt: Attempt,
}

impl Served {
    /// The image `resolved` names, opened for one scope: a registry's
    /// under a new attempt of `blobs`, its manifest written — `None`,
    /// and the attempt left, where the manifest could not be written.
    pub async fn open(resolved: Resolved, store: Option<&Arc<Store>>, blobs: &Arc<Store>, registries: &Arc<RegistryClient>, client_identity: &Identity) -> Option<Served> {
        match resolved {
            Resolved::Store(digest) => Some(Served::Store(Owned {
                store: Arc::clone(store?),
                client_identity: client_identity.clone(),
                digest,
            })),
            Resolved::Registry { host, name, digest, manifest } => {
                let attempt = blobs.enter().await;
                if !matches!(blobs.write_bytes(attempt, &digest, &manifest).await, Ok(true)) {
                    blobs.leave(attempt).await;
                    let _ = blobs.prune().await;
                    return None;
                }
                Some(Served::Registry(Through {
                    blobs: Arc::clone(blobs),
                    registries: Arc::clone(registries),
                    host,
                    name,
                    digest,
                    attempt,
                }))
            }
        }
    }
}

impl oci_store::ServedImage for Served {
    type Blob = Pieces;

    async fn manifest(&self, digest: &str) -> Option<Manifest> {
        let digest = Digest::parse(digest)?;
        match self {
            Served::Store(owned) => owned.manifest(&digest).await,
            Served::Registry(through) => through.manifest(&digest).await,
        }
    }

    async fn blob(&self, digest: &str) -> Option<Pieces> {
        let digest = Digest::parse(digest)?;
        match self {
            Served::Store(owned) => owned.blob(&digest).await,
            Served::Registry(through) => through.blob(&digest).await,
        }
    }

    async fn prove(&self, platform: Option<&str>, nonce: &[u8; 32]) -> Option<[u8; 32]> {
        let platform = match platform {
            Some(platform) => Some(Digest::parse(platform)?),
            None => None,
        };
        match self {
            Served::Store(owned) => owned.prove(platform.as_ref(), nonce).await,
            Served::Registry(through) => through.prove(platform.as_ref(), nonce).await,
        }
    }

    async fn close(&self) {
        if let Served::Registry(through) = self {
            through.close().await;
        }
    }
}

impl Owned {
    /// Whether the image reaches `digest`, while the caller owns it: a
    /// record given up mid-scope serves nothing more.
    async fn reaches(&self, digest: &Digest) -> bool {
        matches!(self.store.owned(&self.client_identity, &self.digest).await, Ok(Some(_)))
            && self.store.reach(&self.digest).await.contains(&digest.to_string())
    }

    /// A manifest the image reaches, as the Store holds it.
    async fn manifest(&self, digest: &Digest) -> Option<Manifest> {
        if !self.reaches(digest).await {
            return None;
        }
        self.store.manifest(digest).await
    }

    /// A blob the image reaches, as the Store holds it.
    async fn blob(&self, digest: &Digest) -> Option<Pieces> {
        if !self.reaches(digest).await {
            return None;
        }
        self.store.pieces(digest).await
    }

    /// The proof over the Store's own bytes, while the caller owns the
    /// image.
    async fn prove(&self, platform: Option<&Digest>, nonce: &[u8; 32]) -> Option<[u8; 32]> {
        if !matches!(self.store.owned(&self.client_identity, &self.digest).await, Ok(Some(_))) {
            return None;
        }
        self.store.prove(nonce, &self.digest, platform).await.ok()?
    }
}

impl Through {
    /// Whether the image reaches `digest`: its manifest; for an index,
    /// every manifest it lists; the config and the layers of each image
    /// manifest reached. An index's manifests the blob store holds are
    /// read first, and only then are the rest fetched through to look.
    async fn reaches(&self, digest: &Digest) -> bool {
        if digest == &self.digest {
            return true;
        }
        let Some(body) = self.blobs.read_blob(&self.digest).await else {
            return false;
        };
        match Parsed::parse(&body) {
            Some(Parsed::Image { config, layers }) => lists(&config, &layers, digest),
            Some(Parsed::Index(entries)) => {
                if entries.iter().any(|entry| &entry.descriptor.digest == digest) {
                    return true;
                }
                let mut absent = Vec::new();
                for entry in &entries {
                    match self.blobs.read_blob(&entry.descriptor.digest).await {
                        Some(body) if image_lists(&body, digest) => return true,
                        Some(_) => {}
                        None => absent.push(&entry.descriptor.digest),
                    }
                }
                for platform in absent {
                    if let Some(body) = self.fetched_manifest(platform).await {
                        if image_lists(&body, digest) {
                            return true;
                        }
                    }
                }
                false
            }
            None => false,
        }
    }

    /// The manifest under `digest`, pinned: the blob store's when it
    /// holds it, else the registry's, found to be the digest's and
    /// written.
    async fn fetched_manifest(&self, digest: &Digest) -> Option<Vec<u8>> {
        if self.blobs.hold(self.attempt, digest).await.is_some() {
            return self.blobs.read_blob(digest).await;
        }
        let body = self.registries.manifest(&self.host, &self.name, &digest.to_string()).await.ok()??;
        if !self.blobs.write_bytes(self.attempt, digest, &body).await.ok()? {
            return None;
        }
        Some(body.to_vec())
    }

    /// A manifest the image reaches, fetched through.
    async fn manifest(&self, digest: &Digest) -> Option<Manifest> {
        if !self.reaches(digest).await {
            return None;
        }
        self.fetched_manifest(digest).await?;
        self.blobs.manifest(digest).await
    }

    /// A blob the image reaches, fetched through: written whole and
    /// verified before its first piece is served.
    async fn blob(&self, digest: &Digest) -> Option<Pieces> {
        if !self.reaches(digest).await {
            return None;
        }
        if self.blobs.hold(self.attempt, digest).await.is_none() {
            let pieces = self.registries.blob(&self.host, &self.name, &digest.to_string()).await.ok()??;
            self.blobs.write_stream(self.attempt, digest, pieces).await.ok()??;
        }
        self.blobs.pieces(digest).await
    }

    /// Rule 4: the image's plan at `platform` brought in under the
    /// scope's attempt, its parts staying pinned for the scope, and the
    /// proof over the blob store's bytes. The cache record is
    /// [`close`](Self::close)'s, as for a part served.
    async fn prove(&self, platform: Option<&Digest>, nonce: &[u8; 32]) -> Option<[u8; 32]> {
        let repository = Repository {
            client: &self.registries,
            host: &self.host,
            name: &self.name,
        };
        let plan = plan_at(&self.blobs, self.attempt, &repository, &self.digest, platform).await.ok()??;
        fill(&self.blobs, self.attempt, &repository, &plan, |_, _| async {}).await.ok()?;
        self.blobs.prove(nonce, &self.digest, platform).await.ok()?
    }

    /// Rule 5: a cache record of a whole image and the trim, then the
    /// attempt left and the blob store pruned.
    async fn close(&self) {
        if self.whole().await {
            if self.blobs.cache(&self.digest).await.is_ok() {
                let _ = self.blobs.make_room(0).await;
            }
        }
        self.blobs.leave(self.attempt).await;
        let _ = self.blobs.prune().await;
    }

    /// Whether every part of the image is present: every part
    /// [`Store::reach`] names held, and for an index at least one of
    /// the manifests it lists among them.
    async fn whole(&self) -> bool {
        let reached = self.blobs.reach(&self.digest).await;
        if let Some(Parsed::Index(entries)) = self.blobs.read_blob(&self.digest).await.as_deref().and_then(Parsed::parse) {
            if !entries.iter().any(|entry| reached.contains(&entry.descriptor.digest.to_string())) {
                return false;
            }
        }
        for part in &reached {
            let Some(part) = Digest::parse(part) else {
                return false;
            };
            if self.blobs.blob_size(&part).await.is_none() {
                return false;
            }
        }
        true
    }
}

/// Whether an image manifest's config or one of its layers is `digest`.
fn lists(config: &Descriptor, layers: &[Descriptor], digest: &Digest) -> bool {
    &config.digest == digest || layers.iter().any(|layer| &layer.digest == digest)
}

/// Whether `body` is an image manifest that lists `digest` as its
/// config or a layer.
fn image_lists(body: &[u8], digest: &Digest) -> bool {
    match Parsed::parse(body) {
        Some(Parsed::Image { config, layers }) => lists(&config, &layers, digest),
        _ => false,
    }
}
