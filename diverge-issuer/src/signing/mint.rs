//! Making a signing key.

use diverge_sdk::keyring::{Key, Keyring};
use rsa::RsaPrivateKey;
use rsa::pkcs1::{EncodeRsaPrivateKey as _, LineEnding};

use super::Error;
use crate::store::Store;
use crate::store::keys::Record;

/// The size of a signing key's modulus, in bits.
const BITS: usize = 2048;

/// Mint a signing key: drawn on the blocking pool, since an RSA key is
/// slow to find; its PEM kept in the keyring as
/// `issuer/signing/<kid>`, first; then its record. The answer is the
/// record.
pub(super) async fn mint(store: &Store, keyring: &Keyring) -> Result<Record, Error> {
    let drawn = tokio::task::spawn_blocking(|| RsaPrivateKey::new(&mut rsa::rand_core::OsRng, BITS));
    let key = drawn.await.map_err(|_| Error::Panicked)?.map_err(Error::Generate)?;
    let pem = key.to_pkcs1_pem(LineEnding::LF).map_err(|error| Error::Pem(String::new(), error.to_string()))?;
    let kid = Key::mint().expose()[..16].to_string();
    let entry = format!("issuer/signing/{kid}");
    keyring.set(&entry, Key::given(pem.to_string())).await.map_err(Error::Keyring)?;
    let record = Record { kid, entry, made: crate::now() };
    store.keys().add(&record).await.map_err(Error::Store)?;
    Ok(record)
}
