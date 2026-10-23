//! Reading a key.

use super::{Error, Key, Keyring};

impl Keyring {
    /// The key held as `name`, or `None` when no entry has that name.
    pub async fn get(&self, name: &str) -> Result<Option<Key>, Error> {
        let key: Option<String> = sqlx::query_scalar("select key from keyring_keys where name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::Query)?;
        Ok(key.map(Key::given))
    }
}
