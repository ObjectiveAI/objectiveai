//! Issuing an authorization code.

use super::{Code, Codes};
use crate::store::Error;

impl Codes<'_> {
    /// Keep `code`, and remove every code that has expired.
    pub async fn add(&self, code: &Code) -> Result<(), Error> {
        let mut transaction = self.store.write().await?;
        sqlx::query("delete from authorization_codes where expires <= ?")
            .bind(crate::now())
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "insert into authorization_codes
                (hash, client, subject, redirect_uri, challenge, scope, audience, nonce, auth_time, expires)
                values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&code.hash)
        .bind(&code.client)
        .bind(&code.subject)
        .bind(&code.redirect_uri)
        .bind(&code.challenge)
        .bind(&code.scope)
        .bind(&code.audience)
        .bind(&code.nonce)
        .bind(code.auth_time)
        .bind(code.expires)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}
