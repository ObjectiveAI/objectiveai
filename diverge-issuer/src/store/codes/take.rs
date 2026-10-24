//! Spending an authorization code.

use super::{Code, Codes};
use crate::store::Error;

/// A code's row, as the database answers it.
type Row = (String, String, String, String, String, String, String, Option<String>, i64, i64);

impl Codes<'_> {
    /// The code whose hash is `hash`, removed as it is read, so that it
    /// is spent whatever its token request turns out to be. `None`
    /// when there is no such code: never issued, spent already, or
    /// removed with its client or its identity. An expired code is
    /// answered as any other; its expiry is the caller's to judge.
    pub async fn take(&self, hash: &str) -> Result<Option<Code>, Error> {
        let row: Option<Row> = sqlx::query_as(
            "delete from authorization_codes where hash = ?
                returning hash, client, subject, redirect_uri, challenge, scope, audience, nonce, auth_time, expires",
        )
        .bind(hash)
        .fetch_optional(&self.store.pool)
        .await?;
        Ok(row.map(|(hash, client, subject, redirect_uri, challenge, scope, audience, nonce, auth_time, expires)| Code {
            hash,
            client,
            subject,
            redirect_uri,
            challenge,
            scope,
            audience,
            nonce,
            auth_time,
            expires,
        }))
    }
}
