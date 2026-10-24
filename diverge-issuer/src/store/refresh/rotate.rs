//! Spending a refresh token for the next.

use super::issue::seconds;
use super::{Chain, Refresh};
use crate::store::Error;

/// What a rotation finds — see the [rules](super).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rotation {
    /// The token was spent, and the next issued: the chain.
    Rotated(Chain),
    /// The token was spent already; the chain is revoked now.
    Reused,
    /// The token is refused, and nothing changed.
    Refused,
}

/// A token's row and its chain's, as a rotation reads them.
type Row = (i64, i64, Option<i64>, String, String, String, String, i64, Option<i64>);

impl Refresh<'_> {
    /// Spend the token whose hash is `hash`, presented by `client`, and
    /// issue the one whose hash is `next` in its place, if the token is
    /// good: issued under `days` days ago, unspent, of an unrevoked
    /// chain that is `client`'s.
    pub async fn rotate(&self, hash: &str, client: &str, next: &str, days: u64) -> Result<Rotation, Error> {
        let now = crate::now();
        let mut transaction = self.store.write().await?;
        let row: Option<Row> = sqlx::query_as(
            "select t.chain, t.made, t.used, c.client, c.subject, c.scope, c.audience, c.auth_time, c.revoked
                from refresh_tokens t join refresh_chains c on c.id = t.chain
                where t.hash = ?",
        )
        .bind(hash)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some((id, made, used, owner, subject, scope, audience, auth_time, revoked)) = row else {
            return Ok(Rotation::Refused);
        };
        if owner != client || revoked.is_some() || made <= now.saturating_sub(seconds(days)) {
            return Ok(Rotation::Refused);
        }
        if used.is_some() {
            sqlx::query("update refresh_chains set revoked = ? where id = ?")
                .bind(now)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            transaction.commit().await?;
            return Ok(Rotation::Reused);
        }
        sqlx::query("update refresh_tokens set used = ? where hash = ?")
            .bind(now)
            .bind(hash)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("insert into refresh_tokens (hash, chain, made) values (?, ?, ?)")
            .bind(next)
            .bind(id)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(Rotation::Rotated(Chain { client: owner, subject, scope, audience, auth_time }))
    }
}
