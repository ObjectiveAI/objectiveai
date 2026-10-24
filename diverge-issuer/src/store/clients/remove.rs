//! Removing a client.

use super::Clients;
use crate::store::Error;

impl Clients<'_> {
    /// Remove the client `id`, and with it every code and refresh
    /// token issued to it. The answer is whether there was such a
    /// client.
    pub async fn remove(&self, id: &str) -> Result<bool, Error> {
        let removed = sqlx::query("delete from clients where id = ?")
            .bind(id)
            .execute(&self.store.pool)
            .await?
            .rows_affected();
        Ok(removed == 1)
    }
}
