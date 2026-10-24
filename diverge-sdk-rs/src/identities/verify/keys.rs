//! An issuer's keys, from the cache or fetched.

use std::sync::Arc;
use std::time::Instant;

use jsonwebtoken::jwk::JwkSet;

use super::{KeySets, VerifyError};

impl KeySets {
    /// `issuer`'s keys, as they stand for `kid` — see the
    /// [rules](super), rule 3: the read lock first, used where its
    /// entry settles `kid`; else the write lock, the entry looked at
    /// again, and only then a fetch, the entry replaced.
    pub(super) async fn keys(&self, issuer: &str, kid: &str) -> Result<Arc<JwkSet>, VerifyError> {
        let lock = self.entry(issuer);
        {
            let read = lock.read().await;
            if let Some(entry) = read.as_ref().filter(|entry| entry.settles(kid, Instant::now())) {
                return Ok(Arc::clone(&entry.keys));
            }
        }
        let mut write = lock.write().await;
        if let Some(entry) = write.as_ref().filter(|entry| entry.settles(kid, Instant::now())) {
            return Ok(Arc::clone(&entry.keys));
        }
        let entry = self.fetch(issuer).await?;
        let keys = Arc::clone(&entry.keys);
        *write = Some(entry);
        Ok(keys)
    }
}
