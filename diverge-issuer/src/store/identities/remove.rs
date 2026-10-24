//! Removing an identity.

use super::Identities;
use crate::store::Error;

impl Identities<'_> {
    /// Remove `subject`, and with it every code and refresh token
    /// issued to it. The answer is whether there was such an identity.
    pub async fn remove(&self, subject: &str) -> Result<bool, Error> {
        let removed = sqlx::query("delete from identities where subject = ?")
            .bind(subject)
            .execute(&self.store.pool)
            .await?
            .rows_affected();
        Ok(removed == 1)
    }
}
