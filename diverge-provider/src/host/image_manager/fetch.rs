//! An image brought into the Store from a supplier — the caller, or a
//! registry the provider uses — blob by blob, only what the Store
//! lacks.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

use super::manifest::{Descriptor, Parsed, select};
use super::{Attempt, Error, Store};
use crate::host::image_registry::Digest;
use crate::host::registry_client::RegistryClient;
use crate::protocol::image_source::ImageSource;
use diverge_sdk::shared::containers::oci;

/// A blob's pieces, as a supplier hands them.
pub type Supplied = Pin<Box<dyn Stream<Item = Bytes> + Send>>;

/// Where an image's bytes are fetched from.
///
/// [`None`] is a digest the supplier does not supply; [`Err`] is a
/// supplier that could not be asked, in its own words.
pub trait Supplier: Send + Sync {
    /// The manifest under `digest`, whole.
    fn manifest(&self, digest: &Digest) -> impl Future<Output = Result<Option<Bytes>, String>> + Send;
    /// The blob under `digest`, piece by piece.
    fn blob(&self, digest: &Digest) -> impl Future<Output = Result<Option<Supplied>, String>> + Send;
}

/// Why an image could not be brought in.
#[derive(Debug)]
pub enum Fetch {
    /// The supplier does not supply a part of it.
    Unavailable,
    /// A manifest that is not one the provider takes, an index with no
    /// manifest for this provider's platform, or a blob of another
    /// size than its descriptor declares.
    Malformed,
    /// Bytes the supplier sent that are not their digest's.
    Corrupt,
    /// The supplier could not be asked.
    Supplier(String),
    /// The Store could not be read or written.
    Store(Error),
}

impl From<Error> for Fetch {
    fn from(error: Error) -> Self {
        Fetch::Store(error)
    }
}

/// An image's shape, its manifests in the Store: the digest it is
/// named by, the platform manifest when that is an index, and the
/// blobs its image manifest references — the config first, then every
/// layer in order, duplicates included — which is `D` past the
/// manifests.
#[derive(Debug, Clone)]
pub struct Plan {
    pub digest: Digest,
    pub platform: Option<Digest>,
    pub blobs: Vec<Descriptor>,
}

impl Plan {
    /// The bytes of the blobs the Store does not hold, each distinct
    /// blob once, as their descriptors declare them: what a push's
    /// capacity is checked against before any of them moves.
    pub async fn missing(&self, store: &Store) -> u64 {
        let mut seen = std::collections::HashSet::new();
        let mut total = 0;
        for blob in &self.blobs {
            if seen.insert(blob.digest.to_string()) && store.blob_size(&blob.digest).await.is_none() {
                total += blob.size;
            }
        }
        total
    }
}

/// The manifests of the image `digest` names, in the Store, pinned for
/// the attempt, and its plan.
///
/// 1. The manifest is the Store's when it holds it, else the
///    supplier's, found to be the digest's and written.
/// 2. An index is resolved to the manifest it lists for this
///    provider's platform — see [`select`] — fetched the same way; an
///    index with none is [`Fetch::Malformed`], and so is a platform
///    manifest that is an index itself.
/// 3. The image manifest's config and layers are the plan's blobs;
///    none of them is fetched here.
pub async fn plan<S: Supplier>(store: &Store, attempt: Attempt, supplier: &S, digest: &Digest) -> Result<Plan, Fetch> {
    let body = manifest(store, attempt, supplier, digest).await?;
    let (platform, image) = match Parsed::parse(&body).ok_or(Fetch::Malformed)? {
        Parsed::Index(entries) => {
            let entry = select(&entries).ok_or(Fetch::Malformed)?;
            let platform = entry.descriptor.digest.clone();
            let body = manifest(store, attempt, supplier, &platform).await?;
            (Some(platform), Parsed::parse(&body).ok_or(Fetch::Malformed)?)
        }
        image => (None, image),
    };
    let Parsed::Image { config, layers } = image else {
        return Err(Fetch::Malformed);
    };
    let mut blobs = Vec::with_capacity(1 + layers.len());
    blobs.push(config);
    blobs.extend(layers);
    Ok(Plan {
        digest: digest.clone(),
        platform,
        blobs,
    })
}

/// One manifest, the Store's or the supplier's, pinned.
async fn manifest<S: Supplier>(store: &Store, attempt: Attempt, supplier: &S, digest: &Digest) -> Result<Vec<u8>, Fetch> {
    store.pin(attempt, digest).await;
    if let Some(body) = store.read_blob(digest).await {
        return Ok(body);
    }
    let body = supplier.manifest(digest).await.map_err(Fetch::Supplier)?.ok_or(Fetch::Unavailable)?;
    if !store.write_bytes(attempt, digest, &body).await? {
        return Err(Fetch::Corrupt);
    }
    Ok(body.to_vec())
}

/// Every blob of the plan the Store does not hold, fetched from the
/// supplier, verified and written, pinned for the attempt — `fetched`
/// told of each with its size, in the plan's order; a blob the Store
/// holds is pinned and not asked for. A blob whose bytes are not its
/// digest's is [`Fetch::Corrupt`]; one not of its declared size,
/// [`Fetch::Malformed`].
pub async fn fill<S, F, T>(store: &Store, attempt: Attempt, supplier: &S, plan: &Plan, mut fetched: F) -> Result<(), Fetch>
where
    S: Supplier,
    F: FnMut(&Descriptor, u64) -> T,
    T: Future<Output = ()>,
{
    for blob in &plan.blobs {
        if store.hold(attempt, &blob.digest).await.is_some() {
            continue;
        }
        let pieces = supplier.blob(&blob.digest).await.map_err(Fetch::Supplier)?.ok_or(Fetch::Unavailable)?;
        let Some(size) = store.write_stream(attempt, &blob.digest, pieces).await? else {
            return Err(Fetch::Corrupt);
        };
        if size != blob.size {
            return Err(Fetch::Malformed);
        }
        fetched(blob, size).await;
    }
    Ok(())
}

/// A nonce for one check: 32 bytes from the operating system's
/// cryptographic source, drawn for this check and used for no other.
pub fn nonce() -> Result<[u8; 32], Error> {
    let mut nonce = [0u8; 32];
    getrandom::fill(&mut nonce).map_err(|error| Error::Random(error.to_string()))?;
    Ok(nonce)
}

/// The nonce check of
/// [`prove`](diverge_sdk::shared::containers::oci::prove), made of the
/// caller over the image `plan` names, whose every part is in the
/// Store: a fresh nonce sent with the digest and — for an index — the
/// platform this provider took from it; the caller's answer compared
/// with the Store's own proof over its own bytes. `true` only for two
/// equal proofs; an empty finish, a malformed answer, a different
/// value, or a part the Store does not hold is `false`.
pub async fn check(store: &Store, source: &ImageSource, plan: &Plan) -> Result<bool, Error> {
    let nonce = nonce()?;
    let request = oci::prove::request::Request {
        nonce,
        digest: plan.digest.to_string(),
        platform: plan.platform.as_ref().map(Digest::to_string),
    };
    let (theirs, ours) = futures_util::future::join(
        source.prove(&request),
        store.prove(&nonce, &plan.digest, plan.platform.as_ref()),
    )
    .await;
    Ok(matches!((theirs, ours?), (Some(theirs), Some(ours)) if theirs == ours))
}

/// The caller, over a run's or a push's scope: its manifests and
/// blobs by digest.
impl Supplier for ImageSource {
    async fn manifest(&self, digest: &Digest) -> Result<Option<Bytes>, String> {
        Ok(ImageSource::manifest(self, &digest.to_string()).await.map(|manifest| manifest.body))
    }

    async fn blob(&self, digest: &Digest) -> Result<Option<Supplied>, String> {
        Ok(ImageSource::blob(self, &digest.to_string()).await.map(|blob| Box::pin(blob) as Supplied))
    }
}

/// One repository of a registry the provider uses, through the
/// [`RegistryClient`].
pub struct Repository<'a> {
    pub client: &'a RegistryClient,
    pub host: &'a str,
    pub name: &'a str,
}

impl Supplier for Repository<'_> {
    async fn manifest(&self, digest: &Digest) -> Result<Option<Bytes>, String> {
        self.client.manifest(self.host, self.name, &digest.to_string()).await
    }

    async fn blob(&self, digest: &Digest) -> Result<Option<Supplied>, String> {
        self.client.blob(self.host, self.name, &digest.to_string()).await
    }
}
