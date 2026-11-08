//! The Store: the blobs, the ownership records, the cache records,
//! and the attempts in flight.

use diverge_sdk::shared::Identity;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use futures_util::{Stream, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt as _;
use tokio::sync::Mutex;

use super::manifest::{Parsed, media_type};
use super::{Error, Pieces, pieces};
use crate::config::images;
use crate::host::image_registry::Digest;
use crate::protocol::image_source::Manifest;
use diverge_sdk::identities::path_name;
use diverge_sdk::provider::endpoints::images::list::server::response::Image;
use diverge_sdk::shared::containers::oci::prove::proof::Prover;
use diverge_sdk::shared::discover::Description;

/// One identity's ownership of one image,
/// `owners/<path_name(issuer)>/<path_name(subject)>/<algorithm>-<hex>.json`:
/// what the blobs do not say of themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    /// The owner's description of the image, if it gave one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    /// The image's size in BYTES: every blob it reaches, each once.
    pub bytes: u64,
    /// When the ownership was gained, in seconds since the epoch.
    pub created: u64,
    /// The Containerfile, verbatim, for an image built here; `None`
    /// for one pushed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub containerfile: Option<String>,
}

/// A cache record, `cache/<algorithm>-<hex>.json`: an image a caller
/// supplied to a run and proved it held, which nobody owns, kept so
/// that its blobs need not cross again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cached {
    /// When the record was written, in seconds since the epoch: the
    /// trim's order, oldest first.
    pub created: u64,
}

/// One attempt to bring an image into the Store — a build, a push, a
/// deploy's pull from the caller — as [`Store::enter`] hands it out:
/// every blob it pins is reachable until it [leaves](Store::leave).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Attempt(u64);

/// The attempts in flight, and what each pinned.
#[derive(Debug, Default)]
struct Attempts {
    next: u64,
    pins: HashMap<u64, HashSet<String>>,
}

/// The Store: the images callers own on this provider, kept as one
/// set of blobs for every owner.
///
/// # The layout, under the `images` section's `path`
///
/// 1. `blobs/<algorithm>/<hex>`: every blob — manifests, configs and
///    layers — content-addressed, ONE copy each, however many owners'
///    images reach it.
/// 2. `owners/<path_name(issuer)>/<path_name(subject)>/<algorithm>-<hex>.json`:
///    one [`Meta`] per image an identity owns, named by its manifest
///    digest; the identity's folders named as the volume store names
///    them, by [`path_name`], so nothing of an identity is hashed and
///    two identities never share a folder.
/// 3. `cache/<algorithm>-<hex>.json`: one [`Cached`] per image a
///    caller supplied to a run and proved, which no identity owns.
/// 4. `tmp/`: a blob on its way in, renamed into `blobs/` once its
///    bytes are found to be its digest's. Emptied when the provider
///    starts.
///
/// # Reachability
///
/// 1. An image reaches its manifest; an index, every manifest it
///    lists that the Store holds and what each of those reaches; an
///    image manifest, its config and its layers.
/// 2. A blob is kept while an ownership record's image, a cache
///    record's image, or an attempt in flight reaches it.
/// 3. [`prune`](Store::prune) removes every other blob: after a
///    delete, after a cache record is trimmed, and after an attempt
///    that failed, which so loses exactly what it brought in and
///    nothing anything else reaches.
/// 4. An attempt pins a blob BEFORE it looks whether the Store holds
///    it, and a prune holds the attempts' lock from its reading of
///    the pins to its last removal: so a blob an attempt found present
///    is never removed under it, and one removed first is found
///    absent and fetched again.
///
/// # Capacity
///
/// Global, every blob counted once. [`make_room`](Store::make_room)
/// trims cache records, oldest first, until the room asked for is
/// there; an ownership record is never trimmed.
#[derive(Debug)]
pub struct Store {
    /// The directory, `<path>`, on this host.
    images: PathBuf,
    /// The most every blob together may take, in BYTES.
    capacity: u64,
    /// Held by every change to a record and by every prune and trim,
    /// so that an ownership's check and its writing are one step and
    /// two prunes never interleave.
    records: Mutex<()>,
    /// The attempts in flight.
    attempts: Mutex<Attempts>,
}

impl Store {
    /// The Store the `images` section names.
    pub fn new(images: &images::Images) -> Self {
        Store {
            images: images.path.clone(),
            capacity: images.capacity,
            records: Mutex::new(()),
            attempts: Mutex::new(Attempts::default()),
        }
    }

    /// The most every blob together may take, in BYTES.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// `tmp/` emptied and every folder of the layout made: what the
    /// provider does when it starts, before anything is in flight.
    pub async fn sweep(&self) -> Result<(), Error> {
        let _ = fs::remove_dir_all(self.tmp_dir()).await;
        for dir in [self.tmp_dir(), self.images.join("blobs"), self.images.join("owners"), self.cache_dir()] {
            fs::create_dir_all(dir).await?;
        }
        Ok(())
    }

    /// Where a blob on its way in is written.
    pub fn tmp_dir(&self) -> PathBuf {
        self.images.join("tmp")
    }

    /// Where the blob under `digest` is: `blobs/<algorithm>/<hex>`.
    pub fn blob_path(&self, digest: &Digest) -> PathBuf {
        self.images.join("blobs").join(digest.algorithm()).join(digest.hex())
    }

    /// Where the identity's ownership records are.
    fn owner_dir(&self, client_identity: &Identity) -> PathBuf {
        self.images
            .join("owners")
            .join(path_name(&client_identity.issuer))
            .join(path_name(&client_identity.subject))
    }

    /// Where the cache records are.
    fn cache_dir(&self) -> PathBuf {
        self.images.join("cache")
    }

    // --- attempts -------------------------------------------------------

    /// A new attempt, pinning nothing yet.
    pub async fn enter(&self) -> Attempt {
        let mut attempts = self.attempts.lock().await;
        let id = attempts.next;
        attempts.next += 1;
        attempts.pins.insert(id, HashSet::new());
        Attempt(id)
    }

    /// `digest` pinned for the attempt: reachable until it leaves.
    pub async fn pin(&self, attempt: Attempt, digest: &Digest) {
        if let Some(pins) = self.attempts.lock().await.pins.get_mut(&attempt.0) {
            pins.insert(digest.to_string());
        }
    }

    /// The attempt over: nothing pinned for it any more. What it
    /// brought in is kept only where a record reaches it, from the
    /// next prune on.
    pub async fn leave(&self, attempt: Attempt) {
        self.attempts.lock().await.pins.remove(&attempt.0);
    }

    // --- blobs ----------------------------------------------------------

    /// The size of the blob under `digest`, or `None` where the Store
    /// holds none.
    pub async fn blob_size(&self, digest: &Digest) -> Option<u64> {
        let found = fs::metadata(self.blob_path(digest)).await.ok()?;
        found.is_file().then(|| found.len())
    }

    /// The blob pinned for the attempt, and its size when the Store
    /// holds it: pinned first, so that a prune never removes what was
    /// found here (see [`Store`], reachability rule 4).
    pub async fn hold(&self, attempt: Attempt, digest: &Digest) -> Option<u64> {
        self.pin(attempt, digest).await;
        self.blob_size(digest).await
    }

    /// The blob under `digest`, read whole and found to be the
    /// digest's; `None` otherwise.
    pub async fn read_blob(&self, digest: &Digest) -> Option<Vec<u8>> {
        let body = fs::read(self.blob_path(digest)).await.ok()?;
        digest.verifies(&body).then_some(body)
    }

    /// The manifest under `digest`, with the media type it is served
    /// as; `None` for a blob the Store does not hold, one that is not
    /// the digest's, and one that is not JSON.
    pub async fn manifest(&self, digest: &Digest) -> Option<Manifest> {
        let body = self.read_blob(digest).await?;
        serde_json::from_slice::<serde_json::Value>(&body).ok()?;
        Some(Manifest {
            media_type: media_type(&body),
            body: Bytes::from(body),
        })
    }

    /// The blob under `digest` as pieces, or `None`.
    pub async fn pieces(&self, digest: &Digest) -> Option<Pieces> {
        pieces(&self.blob_path(digest)).await
    }

    /// `bytes` written as the blob under `digest`, pinned for the
    /// attempt: `false`, and nothing written, for bytes that are not
    /// the digest's.
    pub async fn write_bytes(&self, attempt: Attempt, digest: &Digest, bytes: &[u8]) -> Result<bool, Error> {
        let piece = Bytes::copy_from_slice(bytes);
        Ok(self.write_stream(attempt, digest, futures_util::stream::iter([piece])).await?.is_some())
    }

    /// The pieces written as the blob under `digest`, pinned for the
    /// attempt, and its size; `None`, and nothing written, for pieces
    /// that do not hash to the digest — one ending short among them.
    ///
    /// Written to `tmp/<uuid>` as they come, hashed on the way, and
    /// renamed into `blobs/` once the hash comes out: a blob in
    /// `blobs/` is always whole and its digest's. One the Store holds
    /// already — two attempts bringing one blob in at once — is left
    /// as it is, and the copy removed.
    pub async fn write_stream<S>(&self, attempt: Attempt, digest: &Digest, pieces: S) -> Result<Option<u64>, Error>
    where
        S: Stream<Item = Bytes> + Send,
    {
        self.pin(attempt, digest).await;
        fs::create_dir_all(self.tmp_dir()).await?;
        let tmp = self.tmp_dir().join(uuid::Uuid::new_v4().to_string());
        let outcome = self.write_into(&tmp, digest, pieces).await;
        if !matches!(outcome, Ok(Some(_))) {
            let _ = fs::remove_file(&tmp).await;
        }
        outcome
    }

    /// The writing, into `tmp`, which the caller removes on anything
    /// but a success.
    async fn write_into<S>(&self, tmp: &std::path::Path, digest: &Digest, pieces: S) -> Result<Option<u64>, Error>
    where
        S: Stream<Item = Bytes> + Send,
    {
        let mut file = fs::File::create(tmp).await?;
        let mut hasher = digest.hasher();
        let mut size = 0u64;
        let mut pieces = std::pin::pin!(pieces);
        while let Some(piece) = pieces.next().await {
            hasher.update(&piece);
            size += piece.len() as u64;
            file.write_all(&piece).await?;
        }
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        if !digest.is(&hasher.finish()) {
            return Ok(None);
        }
        let target = self.blob_path(digest);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).await?;
        }
        if fs::metadata(&target).await.is_ok() {
            let _ = fs::remove_file(tmp).await;
        } else {
            fs::rename(tmp, &target).await?;
        }
        Ok(Some(size))
    }

    /// Every blob file the Store holds: its digest, and its size.
    async fn blob_files(&self) -> Result<Vec<(String, PathBuf, u64)>, Error> {
        let mut files = Vec::new();
        let mut algorithms = match fs::read_dir(self.images.join("blobs")).await {
            Ok(algorithms) => algorithms,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(files),
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
                let size = blob.metadata().await?.len();
                files.push((format!("{algorithm_name}:{hex}"), blob.path(), size));
            }
        }
        Ok(files)
    }

    // --- reachability ---------------------------------------------------

    /// Every blob the image named by `digest` reaches, as
    /// `<algorithm>:<hex>` (see [`Store`], reachability rule 1): the
    /// manifest itself always, whether or not the Store holds it.
    pub async fn reach(&self, digest: &Digest) -> HashSet<String> {
        let mut reached = HashSet::from([digest.to_string()]);
        let Some(body) = self.read_blob(digest).await else {
            return reached;
        };
        match Parsed::parse(&body) {
            Some(Parsed::Index(entries)) => {
                for entry in entries {
                    let Some(platform) = self.read_blob(&entry.descriptor.digest).await else {
                        continue;
                    };
                    reached.insert(entry.descriptor.digest.to_string());
                    if let Some(Parsed::Image { config, layers }) = Parsed::parse(&platform) {
                        reached.insert(config.digest.to_string());
                        reached.extend(layers.iter().map(|layer| layer.digest.to_string()));
                    }
                }
            }
            Some(Parsed::Image { config, layers }) => {
                reached.insert(config.digest.to_string());
                reached.extend(layers.iter().map(|layer| layer.digest.to_string()));
            }
            None => {}
        }
        reached
    }

    /// The bytes an image reaches, each blob once: what its record's
    /// `bytes` says.
    pub async fn image_bytes(&self, digest: &Digest) -> u64 {
        let mut total = 0;
        for blob in self.reach(digest).await {
            if let Some(parsed) = Digest::parse(&blob) {
                total += self.blob_size(&parsed).await.unwrap_or(0);
            }
        }
        total
    }

    /// Every image an ownership record or a cache record names.
    async fn roots(&self, with_cache: bool) -> Result<Vec<Digest>, Error> {
        let mut roots = Vec::new();
        let mut issuers = match fs::read_dir(self.images.join("owners")).await {
            Ok(issuers) => Some(issuers),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if let Some(issuers) = &mut issuers {
            while let Some(issuer) = issuers.next_entry().await? {
                let mut subjects = match fs::read_dir(issuer.path()).await {
                    Ok(subjects) => subjects,
                    Err(_) => continue,
                };
                while let Some(subject) = subjects.next_entry().await? {
                    roots.extend(record_digests(subject.path()).await?);
                }
            }
        }
        if with_cache {
            roots.extend(record_digests(self.cache_dir()).await?);
        }
        Ok(roots)
    }

    /// Every blob not reached, removed (see [`Store`], reachability
    /// rules 2 to 4).
    pub async fn prune(&self) -> Result<(), Error> {
        let _records = self.records.lock().await;
        self.prune_locked().await
    }

    /// The prune, with the records' lock held by the caller.
    async fn prune_locked(&self) -> Result<(), Error> {
        let files = self.blob_files().await?;
        let attempts = self.attempts.lock().await;
        let mut kept: HashSet<String> = attempts.pins.values().flatten().cloned().collect();
        for root in self.roots(true).await? {
            kept.extend(self.reach(&root).await);
        }
        for (digest, path, _) in files {
            if !kept.contains(&digest) {
                match fs::remove_file(&path).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
        drop(attempts);
        Ok(())
    }

    // --- capacity -------------------------------------------------------

    /// How many bytes every blob takes together, each counted once.
    pub async fn used(&self) -> Result<u64, Error> {
        Ok(self.blob_files().await?.iter().map(|(_, _, size)| size).sum())
    }

    /// How many bytes a build or a push could still add, as of now:
    /// the capacity less what the ownership records and the attempts
    /// in flight reach, since every cache record would be trimmed for
    /// room. Never below `0`.
    pub async fn room(&self) -> Result<u64, Error> {
        let files = self.blob_files().await?;
        let mut kept: HashSet<String> = self.attempts.lock().await.pins.values().flatten().cloned().collect();
        for root in self.roots(false).await? {
            kept.extend(self.reach(&root).await);
        }
        let held: u64 = files.iter().filter(|(digest, _, _)| kept.contains(digest)).map(|(_, _, size)| size).sum();
        Ok(self.capacity.saturating_sub(held))
    }

    /// Room for `needed` more bytes made, if it can be: while every
    /// blob together and `needed` pass the capacity, the oldest cache
    /// record is removed and the Store pruned. `false` is no cache
    /// record left and still no room; nothing owned is ever trimmed.
    pub async fn make_room(&self, needed: u64) -> Result<bool, Error> {
        let _records = self.records.lock().await;
        loop {
            if self.used().await?.saturating_add(needed) <= self.capacity {
                return Ok(true);
            }
            let Some(oldest) = self.oldest_cached().await? else {
                return Ok(false);
            };
            remove(self.cache_dir().join(record_file(&oldest))).await?;
            self.prune_locked().await?;
        }
    }

    /// The cache record written longest ago, if any.
    async fn oldest_cached(&self) -> Result<Option<Digest>, Error> {
        let mut oldest: Option<(u64, Digest)> = None;
        for digest in record_digests(self.cache_dir()).await? {
            let Ok(bytes) = fs::read(self.cache_dir().join(record_file(&digest))).await else {
                continue;
            };
            let created = serde_json::from_slice::<Cached>(&bytes).map(|cached| cached.created).unwrap_or(0);
            if oldest.as_ref().is_none_or(|(at, _)| created < *at) {
                oldest = Some((created, digest));
            }
        }
        Ok(oldest.map(|(_, digest)| digest))
    }

    // --- records --------------------------------------------------------

    /// The identity's ownership record of `digest`, or `None`.
    pub async fn owned(&self, client_identity: &Identity, digest: &Digest) -> Result<Option<Meta>, Error> {
        read_record(self.owner_dir(client_identity).join(record_file(digest))).await
    }

    /// Whether `digest` is in the identity's Store: a digest string
    /// that does not parse never is.
    pub async fn owns(&self, client_identity: &Identity, digest: &str) -> bool {
        match Digest::parse(digest) {
            Some(digest) => matches!(self.owned(client_identity, &digest).await, Ok(Some(_))),
            None => false,
        }
    }

    /// The identity's ownership of `digest` recorded, `meta` its
    /// record: `false`, and nothing written, where it owns the digest
    /// already. The check and the write are one step under the
    /// records' lock. The cache record of the digest, when there is
    /// one, goes: an owned image is no cache's.
    pub async fn own(&self, client_identity: &Identity, digest: &Digest, meta: &Meta) -> Result<bool, Error> {
        let _records = self.records.lock().await;
        let path = self.owner_dir(client_identity).join(record_file(digest));
        if fs::metadata(&path).await.is_ok() {
            return Ok(false);
        }
        write_record(&path, meta).await?;
        remove(self.cache_dir().join(record_file(digest))).await?;
        Ok(true)
    }

    /// The identity's ownership of `digest` removed and the Store
    /// pruned: `false` where it owned no such image.
    pub async fn disown(&self, client_identity: &Identity, digest: &Digest) -> Result<bool, Error> {
        let _records = self.records.lock().await;
        let path = self.owner_dir(client_identity).join(record_file(digest));
        if fs::metadata(&path).await.is_err() {
            return Ok(false);
        }
        remove(path).await?;
        self.prune_locked().await?;
        Ok(true)
    }

    /// The description of the identity's image set, or cleared with
    /// `None`: `None` where it owns no such image, else whether the
    /// description changed.
    pub async fn describe(&self, client_identity: &Identity, digest: &Digest, description: Option<Description>) -> Result<Option<bool>, Error> {
        let _records = self.records.lock().await;
        let path = self.owner_dir(client_identity).join(record_file(digest));
        let Some(mut meta) = read_record::<Meta>(path.clone()).await? else {
            return Ok(None);
        };
        if meta.description == description {
            return Ok(Some(false));
        }
        meta.description = description;
        write_record(&path, &meta).await?;
        Ok(Some(true))
    }

    /// Every image the identity owns, as a listing reports them.
    pub async fn list(&self, client_identity: &Identity) -> Result<Vec<Image>, Error> {
        let dir = self.owner_dir(client_identity);
        let mut images = Vec::new();
        for digest in record_digests(dir.clone()).await? {
            let Some(meta) = read_record::<Meta>(dir.join(record_file(&digest))).await? else {
                continue;
            };
            images.push(Image {
                digest: digest.to_string(),
                description: meta.description,
                bytes: meta.bytes,
                created: meta.created,
            });
        }
        Ok(images)
    }

    /// Whether any identity owns `digest`.
    pub async fn owned_by_anyone(&self, digest: &Digest) -> Result<bool, Error> {
        Ok(self.roots(false).await?.contains(digest))
    }

    /// A cache record of `digest` written, unless an identity owns it
    /// — a record of its own keeps its blobs already.
    pub async fn cache(&self, digest: &Digest) -> Result<(), Error> {
        let _records = self.records.lock().await;
        if self.roots(false).await?.contains(digest) {
            return Ok(());
        }
        write_record(&self.cache_dir().join(record_file(digest)), &Cached { created: now() }).await
    }

    /// Whether the identity's Store reaches `digest`: an image it
    /// owns, or a blob one of them reaches — what the `images::oci`
    /// scope serves, and nothing else.
    pub async fn reaches_owned(&self, client_identity: &Identity, digest: &Digest) -> bool {
        let Ok(owned) = record_digests(self.owner_dir(client_identity)).await else {
            return false;
        };
        let wanted = digest.to_string();
        for image in owned {
            if self.reach(&image).await.contains(&wanted) {
                return true;
            }
        }
        false
    }

    // --- the proof ------------------------------------------------------

    /// The proof of possession over the Store's own bytes of the image
    /// `digest` names, keyed by `nonce` — see
    /// [`prove`](diverge_sdk::shared::containers::oci::prove).
    ///
    /// The rules:
    ///
    /// 1. The manifest under `digest` is held and parses, or `None`.
    /// 2. An index takes a `platform`, which is one of the manifests it
    ///    lists and is held and parses as an image manifest; an image
    ///    manifest takes none. Either broken is `None`: a proof is
    ///    never computed over an index alone.
    /// 3. `D` is the manifest, then the platform manifest, then the
    ///    config, then every layer in the order listed, duplicates
    ///    included, every part the Store's own file; a part the Store
    ///    does not hold is `None`.
    pub async fn prove(&self, nonce: &[u8; 32], digest: &Digest, platform: Option<&Digest>) -> Result<Option<[u8; 32]>, Error> {
        let Some(body) = self.read_blob(digest).await else {
            return Ok(None);
        };
        let mut prover = Prover::new(nonce);
        prover.update(&body);
        let image = match (Parsed::parse(&body), platform) {
            (Some(Parsed::Index(entries)), Some(platform)) => {
                if !entries.iter().any(|entry| &entry.descriptor.digest == platform) {
                    return Ok(None);
                }
                let Some(platform_body) = self.read_blob(platform).await else {
                    return Ok(None);
                };
                prover.update(&platform_body);
                Parsed::parse(&platform_body)
            }
            (Some(image @ Parsed::Image { .. }), None) => Some(image),
            _ => return Ok(None),
        };
        let Some(Parsed::Image { config, layers }) = image else {
            return Ok(None);
        };
        for part in std::iter::once(&config).chain(layers.iter()) {
            let Some(pieces) = self.pieces(&part.digest).await else {
                return Ok(None);
            };
            let mut pieces = pieces;
            while let Some(piece) = pieces.next().await {
                prover.update(&piece);
            }
        }
        Ok(Some(prover.finish()))
    }
}

/// A record's file name: `<algorithm>-<hex>.json`.
fn record_file(digest: &Digest) -> String {
    format!("{}-{}.json", digest.algorithm(), digest.hex())
}

/// Every record in `dir`, by the digest its file is named for; a file
/// that is not a record's is passed over, and a folder that is not
/// there holds none.
async fn record_digests(dir: PathBuf) -> Result<Vec<Digest>, Error> {
    let mut entries = match fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut digests = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let Ok(file) = entry.file_name().into_string() else {
            continue;
        };
        let Some((algorithm, hex)) = file.strip_suffix(".json").and_then(|stem| stem.split_once('-')) else {
            continue;
        };
        if let Some(digest) = Digest::parse(&format!("{algorithm}:{hex}")) {
            digests.push(digest);
        }
    }
    Ok(digests)
}

/// The record at `path`, or `None` where there is none.
async fn read_record<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<Option<T>, Error> {
    match fs::read(&path).await {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// A record written whole or not at all: to a sibling, then renamed
/// over.
async fn write_record<T: Serialize>(path: &std::path::Path, record: &T) -> Result<(), Error> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).await?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec(record)?).await?;
    fs::rename(&temporary, path).await?;
    Ok(())
}

/// A file removed; one already gone is removed.
async fn remove(path: PathBuf) -> Result<(), Error> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Now, in seconds since the epoch.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}
