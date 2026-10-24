//! What a refresh token's chain grants, without spending it.

use super::{Chain, Refresh};
use crate::store::Error;

impl Refresh<'_> {
    /// The chain of the token whose hash is `hash`, whatever the
    /// token's state: what a refresh request's scope and resource are
    /// checked against before the token is spent. A chain's grant
    /// never changes, so what this reads is what the rotation spends.
    pub async fn peek(&self, hash: &str) -> Result<Option<Chain>, Error> {
        let row: Option<(String, String, String, String, i64)> = sqlx::query_as(
            "select c.client, c.subject, c.scope, c.audience, c.auth_time
                from refresh_tokens t join refresh_chains c on c.id = t.chain
                where t.hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.store.pool)
        .await?;
        Ok(row.map(|(client, subject, scope, audience, auth_time)| Chain { client, subject, scope, audience, auth_time }))
    }
}
