//! Making a signing key, and rotating to a new one.

use rsa::RsaPrivateKey;
use rsa::pkcs1::{EncodeRsaPrivateKey as _, LineEnding};

use super::{Error, Live, Signing, SigningKey};
use crate::store::Store;
use crate::store::keys::Record;
use crate::tokens;

/// The size of a signing key's modulus, in bits.
const BITS: usize = 2048;

/// Draw a signing key — on the blocking pool, since an RSA key is slow
/// to find — as a record, PEM and all, written nowhere yet.
async fn draw() -> Result<Record, Error> {
    let drawn = tokio::task::spawn_blocking(|| RsaPrivateKey::new(&mut rsa::rand_core::OsRng, BITS));
    let key = drawn.await.map_err(|_| Error::Panicked)?.map_err(Error::Generate)?;
    let pem = key.to_pkcs1_pem(LineEnding::LF).map_err(|error| Error::Pem(String::new(), error.to_string()))?;
    let kid = tokens::mint()[..16].to_string();
    Ok(Record { kid, pem: pem.to_string(), made: crate::now() })
}

/// Mint a signing key and record it: what a start with no key does.
/// The answer is the record.
pub async fn mint(store: &Store) -> Result<Record, Error> {
    let record = draw().await?;
    store.keys().add(&record).await.map_err(Error::Store)?;
    Ok(record)
}

/// What a [`rotate`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rotated {
    /// The new key's record.
    pub key: Record,
    /// How many old keys were removed.
    pub removed: u64,
    /// How many refresh chains were revoked: none unless signing out
    /// was asked.
    pub signed_out: u64,
}

/// Rotate the signing key — see the [rules](super), rule 5. In order:
///
/// 1. A new key is drawn, and made ready to sign and to publish — the
///    [`Live`] keys of it alone. Nothing is written yet, so a failure
///    here changes nothing.
/// 2. In one transaction, the new key is recorded, every other removed,
///    and, with `sign_out`, every refresh chain revoked. A failure
///    rolls the whole of it back.
///
/// The answer is what was done and the new live keys, which a running
/// issuer swaps in — a step that cannot fail — so that its memory and
/// its database never disagree.
pub async fn rotate(store: &Store, sign_out: bool) -> Result<(Rotated, Live), Error> {
    let record = draw().await?;
    let live = Live::new(Signing::of(vec![SigningKey::from_pem(&record.kid, &record.pem)?]))?;
    let (removed, signed_out) = store.keys().replace(&record, sign_out).await.map_err(Error::Store)?;
    Ok((Rotated { key: record, removed, signed_out }, live))
}
