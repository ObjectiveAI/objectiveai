//! The keys, read at the start.

use diverge_sdk::keyring::Keyring;
use openidconnect::core::CoreJsonWebKeySet;

use super::{Error, SigningKey, mint::mint};
use crate::store::Store;

/// Every unretired signing key, the newest last: the one that signs.
pub struct Signing {
    keys: Vec<SigningKey>,
}

impl Signing {
    /// The unretired keys, each read from the keyring by its record's
    /// entry — one minted first when there is none. A record whose
    /// entry the keyring does not hold is [`Error::Missing`].
    pub async fn load(store: &Store, keyring: &Keyring) -> Result<Signing, Error> {
        let mut records = store.keys().unretired().await.map_err(Error::Store)?;
        if records.is_empty() {
            records.push(mint(store, keyring).await?);
        }
        let mut keys = Vec::with_capacity(records.len());
        for record in &records {
            let pem = keyring.get(&record.entry).await.map_err(Error::Keyring)?.ok_or_else(|| Error::Missing(record.entry.clone()))?;
            keys.push(SigningKey::from_pem(&record.kid, pem.expose())?);
        }
        Ok(Signing { keys })
    }

    /// The key that signs: the newest.
    pub fn current(&self) -> &SigningKey {
        // `load` answers at least one key, and the list is never
        // changed after.
        &self.keys[self.keys.len() - 1]
    }

    /// The key whose id is `kid`, if it is unretired.
    pub fn find(&self, kid: &str) -> Option<&SigningKey> {
        self.keys.iter().find(|key| key.kid == kid)
    }

    /// The key set: every unretired key's public half.
    pub fn key_set(&self) -> CoreJsonWebKeySet {
        CoreJsonWebKeySet::new(self.keys.iter().map(|key| key.public.clone()).collect())
    }
}
