//! Revoking refresh chains.

use super::Refresh;
use crate::store::Error;

impl Refresh<'_> {
    /// Revoke the chain of the token whose hash is `hash`. The answer
    /// is whether one was revoked now.
    pub async fn revoke(&self, hash: &str) -> Result<bool, Error> {
        let revoked = sqlx::query(
            "update refresh_chains set revoked = ?
                where revoked is null
                and id = (select chain from refresh_tokens where hash = ?)",
        )
        .bind(crate::now())
        .bind(hash)
        .execute(&self.store.pool)
        .await?
        .rows_affected();
        Ok(revoked == 1)
    }

}
