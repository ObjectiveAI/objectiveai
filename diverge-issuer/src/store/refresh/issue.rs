//! Beginning a chain.

use super::{Chain, Refresh};
use crate::store::Error;

impl Refresh<'_> {
    /// Begin the chain `chain`, its first token the one whose hash is
    /// `hash`, and remove every token older than `days` days.
    pub async fn issue(&self, chain: &Chain, hash: &str, days: u64) -> Result<(), Error> {
        let now = crate::now();
        let mut transaction = self.store.write().await?;
        sqlx::query("delete from refresh_tokens where made <= ?")
            .bind(now.saturating_sub(seconds(days)))
            .execute(&mut *transaction)
            .await?;
        let id: i64 = sqlx::query_scalar(
            "insert into refresh_chains (subject, scope, audience, auth_time, made)
                values (?, ?, ?, ?, ?) returning id",
        )
        .bind(&chain.subject)
        .bind(&chain.scope)
        .bind(&chain.audience)
        .bind(chain.auth_time)
        .bind(now)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query("insert into refresh_tokens (hash, chain, made) values (?, ?, ?)")
            .bind(hash)
            .bind(id)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}

/// `days` in seconds.
pub(super) fn seconds(days: u64) -> i64 {
    i64::try_from(days.saturating_mul(86_400)).unwrap_or(i64::MAX)
}
