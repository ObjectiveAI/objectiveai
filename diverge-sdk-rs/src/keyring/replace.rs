//! Keeping a key as a name, whatever the name held.

use super::{Error, Key, Keyring};

impl Keyring {
    /// Keep `key` as `name`: the entry is made if it is absent, and
    /// its key replaced if it is there. What a server that minted a
    /// key for a credential it has just committed calls, so the entry
    /// says what the record does.
    pub async fn replace(&self, name: &str, key: Key) -> Result<(), Error> {
        sqlx::query(
            "insert into keyring_keys (name, key) values (?, ?)
             on conflict (name) do update set key = excluded.key, made = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(name)
        .bind(key.expose())
        .execute(&self.pool)
        .await
        .map_err(Error::Query)?;
        Ok(())
    }
}
