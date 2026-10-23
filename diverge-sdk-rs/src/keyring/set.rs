//! Keeping a key someone was given.

use super::{Error, Key, Keyring};

impl Keyring {
    /// Keep `key` as `name`. A name already held is
    /// [`Error::Exists`], and its key is left as it is.
    pub async fn set(&self, name: &str, key: Key) -> Result<(), Error> {
        let done = sqlx::query("insert into keyring_keys (name, key) values (?, ?) on conflict (name) do nothing")
            .bind(name)
            .bind(key.expose())
            .execute(&self.pool)
            .await
            .map_err(Error::Query)?;
        match done.rows_affected() {
            0 => Err(Error::Exists(name.to_string())),
            _ => Ok(()),
        }
    }
}
