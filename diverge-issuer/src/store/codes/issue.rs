//! Issuing an authorization code.

use super::{Bound, Codes, LIFETIME};
use crate::store::Error;

impl Codes<'_> {
    /// Keep the code whose SHA-256 is `hash`, bound to `bound`, good for
    /// [`LIFETIME`] seconds — every expired code removed first.
    pub async fn issue(&self, hash: &str, bound: &Bound) -> Result<(), Error> {
        let now = crate::now();
        let mut transaction = self.store.write().await?;
        sqlx::query("delete from authorization_codes where expires <= ?")
            .bind(now)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "insert into authorization_codes (hash, subject, redirect_uri, challenge, nonce, scope, audience, auth_time, expires)
                values (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(hash)
        .bind(&bound.subject)
        .bind(&bound.redirect_uri)
        .bind(&bound.challenge)
        .bind(&bound.nonce)
        .bind(&bound.scope)
        .bind(&bound.audience)
        .bind(bound.auth_time)
        .bind(now.saturating_add(LIFETIME))
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}
