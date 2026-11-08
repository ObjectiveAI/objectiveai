//! Where a run's image is taken from: the caller's Store, a registry,
//! or the caller — each step of the order
//! [`obtain`](super::deploy::obtain) follows.

use std::path::Path;

use diverge_sdk::shared::Identity;
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;

use super::deploy::pull;
use super::{ContainerDeployer, Error};
use crate::host::image_manager::fetch::{Fetch, check, fill, plan};
use crate::host::image_manager::{Attempt, Store};
use crate::host::image_registry::{Digest, Origin};
use crate::host::tools::podman;
use crate::protocol::caller::Caller;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::shared::containers::response::Source;

/// The label a pull from the provider's own registry names the image
/// by when the caller answered no name that is a repository path: the
/// segment is never read.
const LABEL: &str = "image";

/// Step 1: the caller's own image, when it owns the digest, pulled
/// from the provider's own registry under a token minted for the Store
/// and revoked as the pull answers —
/// `127.0.0.1:<port>/<token>/image@<digest>` — a confirmation and no
/// bytes where the cache holds the copy still. `None` where the caller
/// owns no such digest.
pub(super) async fn from_store(deployer: &ContainerDeployer, client_identity: &Identity, digest: &Digest) -> Result<Option<(Source, String)>, Error> {
    if deployer.store.owned(client_identity, digest).await.map_err(Error::Store)?.is_none() {
        return Ok(None);
    }
    let id = through(deployer, Origin::Store, LABEL, digest).await?;
    Ok(Some((Source::Store, id)))
}

/// Step 2: the cache's copy by digest, held under a registry's name —
/// one pulled from the provider's own registry, named under
/// `127.0.0.1:` alone, is passed over — run as its id with nothing
/// pulled; else every referenced registry the configuration lists,
/// asked at once as `<registry>/<name>@<digest>` with the credential
/// listed — a reference naming a registry not listed is ignored — and
/// the first to answer that it serves the image pulled from, the rest
/// not waited for. `None` where no registry serves it.
pub(super) async fn from_registry(deployer: &ContainerDeployer, image: &Image) -> Result<Option<(Source, String)>, Error> {
    if let Some((id, name)) = podman::image_by_digest(&image.digest).await.map_err(Error::Podman)? {
        let reference = format!("{}@{}", repository(&name), image.digest);
        return Ok(Some((Source::Registry { reference }, id)));
    }
    let mut asked = FuturesUnordered::new();
    for reference in &image.references {
        if deployer.podman.registries.iter().any(|registry| registry.host == reference.registry) {
            asked.push(in_registry(deployer.auth_file(), &reference.registry, &reference.name, &image.digest));
        }
    }
    let mut found = None;
    while let Some(answered) = asked.next().await {
        if let Some(reference) = answered? {
            found = Some(reference);
            break;
        }
    }
    drop(asked);
    let Some(reference) = found else {
        return Ok(None);
    };
    let id = pull(deployer, &reference, false).await?;
    Ok(Some((Source::Registry { reference }, id)))
}

/// One referenced registry asked whether it serves the image under the
/// reference's name: the reference when it does.
async fn in_registry(auth_file: &Path, host: &str, name: &str, digest: &str) -> Result<Option<String>, Error> {
    let reference = format!("{host}/{name}@{digest}");
    let found = podman::manifest_exists(auth_file, &reference).await.map_err(Error::Podman)?;
    Ok(found.then_some(reference))
}

/// A name the cache holds an image under, less its tag: the
/// repository a reference by digest names.
fn repository(name: &str) -> &str {
    let segment = name.rfind('/').map_or(0, |slash| slash + 1);
    match name[segment..].find(':') {
        Some(colon) => &name[..segment + colon],
        None => name,
    }
}

/// Step 3: the caller, asked whether it holds the image — `None` where
/// it does not — and then, under an attempt of the Store's:
///
/// 1. The image's plan: its manifest, and for an index the manifest
///    for this provider's platform, the Store's or the caller's,
///    verified and kept.
/// 2. Every blob the Store lacks fetched from the caller, verified and
///    kept; nothing it holds is asked for.
/// 3. The nonce check, over the Store's own bytes: a failure is
///    [`Error::ProofFailed`], and nothing has reached podman.
/// 4. The pull, through a token minted for the caller and read
///    through — every part already in the Store, so nothing more
///    moves — `127.0.0.1:<port>/<token>/<name>@<digest>`, `name` the
///    caller's answer when it is a repository path, else `image`.
/// 5. A cache record of the image, unless an identity owns it, and
///    the cache trimmed to the capacity, oldest first.
///
/// Whatever the end, the attempt leaves and the Store is pruned: a
/// failure loses every blob it brought in that nothing else reaches.
/// A part the caller said it held and then did not supply is
/// [`Error::Unavailable`]; a manifest the provider does not take, or
/// bytes that are not their digest's, [`Error::Malformed`].
pub(super) async fn from_client(deployer: &ContainerDeployer, caller: &Caller, digest: &Digest) -> Result<Option<(Source, String)>, Error> {
    let Some(name) = caller.holds().await else {
        return Ok(None);
    };
    let label = if name_ok(&name) { name } else { LABEL.to_string() };
    let store = &deployer.store;
    let attempt = store.enter().await;
    let outcome = taken(deployer, store, attempt, caller, &label, digest).await;
    store.leave(attempt).await;
    let _ = store.prune().await;
    outcome.map(|id| Some((Source::Client, id)))
}

/// Steps 1 to 5 of [`from_client`], under `attempt`.
async fn taken(deployer: &ContainerDeployer, store: &Store, attempt: Attempt, caller: &Caller, label: &str, digest: &Digest) -> Result<String, Error> {
    let fetched = match plan(store, attempt, caller.source(), digest).await {
        Ok(plan) => fill(store, attempt, caller.source(), &plan, |_, _| async {}).await.map(|()| plan),
        Err(fetched) => Err(fetched),
    };
    let plan = match fetched {
        Ok(plan) => plan,
        Err(Fetch::Unavailable | Fetch::Supplier(_)) => {
            return Err(Error::Unavailable {
                digest: digest.to_string(),
            });
        }
        Err(Fetch::Malformed | Fetch::Corrupt) => {
            return Err(Error::Malformed {
                digest: digest.to_string(),
            });
        }
        Err(Fetch::Store(error)) => return Err(Error::Store(error)),
    };
    if !check(store, caller.source(), caller.image(), &plan).await.map_err(Error::Store)? {
        return Err(Error::ProofFailed);
    }
    let origin = Origin::Client {
        source: caller.source().clone(),
        attempt,
    };
    let id = through(deployer, origin, label, digest).await?;
    store.cache(digest).await.map_err(Error::Store)?;
    store.make_room(0).await.map_err(Error::Store)?;
    Ok(id)
}

/// A pull from the provider's own registry, through a token minted for
/// `origin` for this one pull and revoked as it answers, whichever
/// way: `127.0.0.1:<port>/<token>/<label>@<digest>`, at the port
/// podman reaches the registry by.
async fn through(deployer: &ContainerDeployer, origin: Origin, label: &str, digest: &Digest) -> Result<String, Error> {
    let token = deployer.registry.mint(origin).map_err(Error::Registry)?;
    let reference = format!("{}{}/{token}/{label}@{digest}", podman::CLIENT_PREFIX, deployer.registry_port);
    let pulled = pull(deployer, &reference, true).await;
    deployer.registry.revoke(&token);
    pulled
}

/// Whether `name` is a repository path as the OCI distribution
/// specification has one: one or more segments joined by `/`, each
/// lowercase letters and digits with single `.`, `_`, `__` or `-`
/// runs between them. Anything else — an empty segment, `..`, an
/// uppercase letter, a space — is refused, since the name is put
/// into a URL by concatenation and never normalized.
pub fn name_ok(name: &str) -> bool {
    !name.is_empty() && name.split('/').all(segment_ok)
}

/// One segment of a repository path: names joined by separators,
/// where a name is lowercase letters and digits and a separator is
/// `.`, `_`, `__` or a run of `-`, never first, never last, never
/// two in a row.
fn segment_ok(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    let name = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    let mut index = 0;
    while index < bytes.len() {
        if name(bytes[index]) {
            index += 1;
            continue;
        }
        if index == 0 {
            return false;
        }
        match bytes[index] {
            b'.' => index += 1,
            b'_' => {
                index += 1;
                if bytes.get(index) == Some(&b'_') {
                    index += 1;
                }
            }
            b'-' => {
                while bytes.get(index) == Some(&b'-') {
                    index += 1;
                }
            }
            _ => return false,
        }
        if !bytes.get(index).is_some_and(|&byte| name(byte)) {
            return false;
        }
    }
    !bytes.is_empty()
}
