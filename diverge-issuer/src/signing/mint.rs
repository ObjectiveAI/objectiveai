//! Making a signing key, and rotating to a new one.

use rsa::RsaPrivateKey;
use rsa::pkcs1::{EncodeRsaPrivateKey as _, LineEnding};

use super::Error;
use crate::store::Store;
use crate::store::keys::Record;
use crate::tokens;

/// The size of a signing key's modulus, in bits.
const BITS: usize = 2048;

/// Mint a signing key — drawn on the blocking pool, since an RSA key is
/// slow to find — and record it, PEM and all. The answer is the
/// record.
pub async fn mint(store: &Store) -> Result<Record, Error> {
    let drawn = tokio::task::spawn_blocking(|| RsaPrivateKey::new(&mut rsa::rand_core::OsRng, BITS));
    let key = drawn.await.map_err(|_| Error::Panicked)?.map_err(Error::Generate)?;
    let pem = key.to_pkcs1_pem(LineEnding::LF).map_err(|error| Error::Pem(String::new(), error.to_string()))?;
    let kid = tokens::mint()[..16].to_string();
    let record = Record { kid, pem: pem.to_string(), made: crate::now() };
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

/// Rotate the signing key — see the [rules](super), rule 5: a new key
/// minted, the old removed, and, with `sign_out`, every refresh chain
/// revoked.
pub async fn rotate(store: &Store, sign_out: bool) -> Result<Rotated, Error> {
    let key = mint(store).await?;
    let removed = store.keys().remove_all_but(&key.kid).await.map_err(Error::Store)?;
    let signed_out = if sign_out { store.refresh().revoke_all().await.map_err(Error::Store)? } else { 0 };
    Ok(Rotated { key, removed, signed_out })
}
