//! Replacing a key.

use super::{Error, Key, Keyring};

impl Keyring {
    /// Replace the key held as `name` with a new one and answer it.
    /// A name not held is [`Error::Missing`].
    pub async fn rotate(&self, name: &str) -> Result<Key, Error> {
        let key = Key::mint();
        let done = sqlx::query(
            "update keyring_keys set key = ?, made = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') where name = ?",
        )
        .bind(key.expose())
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(Error::Query)?;
        match done.rows_affected() {
            0 => Err(Error::Missing(name.to_string())),
            _ => Ok(key),
        }
    }
}
