//! What the keyring holds, by name.

use super::{Entry, Error, Keyring};

impl Keyring {
    /// Every entry, by name, with when its key was made. Never a key.
    pub async fn list(&self) -> Result<Vec<Entry>, Error> {
        let rows: Vec<(String, String)> = sqlx::query_as("select name, made from keyring_keys order by name")
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Query)?;
        Ok(rows.into_iter().map(|(name, made)| Entry { name, made }).collect())
    }
}
