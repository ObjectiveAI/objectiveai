//! Spending an authorization code.

use super::{Bound, Codes};
use crate::store::Error;

/// A code's row, as it is read.
type Row = (String, String, String, Option<String>, String, String, i64, i64);

impl Codes<'_> {
    /// The code whose SHA-256 is `hash`, removed as it is read, in one
    /// transaction: `None` when there is no such code, or it has
    /// expired. A code is taken at most once.
    pub async fn take(&self, hash: &str) -> Result<Option<Bound>, Error> {
        let mut transaction = self.store.write().await?;
        let row: Option<Row> = sqlx::query_as(
            "select subject, redirect_uri, challenge, nonce, scope, audience, auth_time, expires
                from authorization_codes where hash = ?",
        )
        .bind(hash)
        .fetch_optional(&mut *transaction)
        .await?;
        sqlx::query("delete from authorization_codes where hash = ?")
            .bind(hash)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(row.and_then(|(subject, redirect_uri, challenge, nonce, scope, audience, auth_time, expires)| {
            (expires > crate::now()).then_some(Bound { subject, redirect_uri, challenge, nonce, scope, audience, auth_time })
        }))
    }
}
