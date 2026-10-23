//! Making a key.

use super::{Error, Key, Keyring};

impl Keyring {
    /// Mint a new key as `name` and answer it. A name already held is
    /// [`Error::Exists`], and its key is left as it is.
    pub async fn mint(&self, name: &str) -> Result<Key, Error> {
        let key = Key::mint();
        self.set(name, key.clone()).await?;
        Ok(key)
    }
}
