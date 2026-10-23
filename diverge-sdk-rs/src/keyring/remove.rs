//! Removing an entry.

use super::{Error, Keyring};

impl Keyring {
    /// Remove the entry `name` and its key. Answers whether there was
    /// one.
    pub async fn remove(&self, name: &str) -> Result<bool, Error> {
        let done = sqlx::query("delete from keyring_keys where name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(Error::Query)?;
        Ok(done.rows_affected() > 0)
    }
}
