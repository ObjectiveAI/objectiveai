//! Revoking a chain.

use super::Refresh;
use crate::store::Error;

impl Refresh<'_> {
    /// Revoke the chain of the token whose hash is `hash`, if the chain
    /// is `client`'s. The answer is whether one was revoked now.
    pub async fn revoke(&self, hash: &str, client: &str) -> Result<bool, Error> {
        let revoked = sqlx::query(
            "update refresh_chains set revoked = ?
                where revoked is null and client = ?
                and id = (select chain from refresh_tokens where hash = ?)",
        )
        .bind(crate::now())
        .bind(client)
        .bind(hash)
        .execute(&self.store.pool)
        .await?
        .rows_affected();
        Ok(revoked == 1)
    }
}
