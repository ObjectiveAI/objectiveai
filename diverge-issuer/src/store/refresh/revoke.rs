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

    /// Revoke every chain there is: every identity signs in again. The
    /// answer is how many were revoked now.
    pub async fn revoke_all(&self) -> Result<u64, Error> {
        let revoked = sqlx::query("update refresh_chains set revoked = ? where revoked is null")
            .bind(crate::now())
            .execute(&self.store.pool)
            .await?
            .rows_affected();
        Ok(revoked)
    }
}
