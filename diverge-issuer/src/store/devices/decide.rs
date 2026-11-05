//! The operator's decision on a device code.

use super::Devices;
use crate::store::Error;

impl Devices<'_> {
    /// Approve the pending device code `user_code` is, as the subject it
    /// asked for: the subject made an identity first if it was not one,
    /// and the code approved for it now, in one transaction. The answer
    /// is that subject, or `None` when there was no such code, pending
    /// and not expired.
    pub async fn approve(&self, user_code: &str) -> Result<Option<String>, Error> {
        let now = crate::now();
        let mut transaction = self.store.write().await?;
        let hint: Option<(String,)> = sqlx::query_as(
            "select login_hint from device_codes where user_code = ? and state = 'pending' and expires > ?",
        )
        .bind(user_code)
        .bind(now)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some((subject,)) = hint else {
            return Ok(None);
        };
        sqlx::query("insert or ignore into identities (subject, made) values (?, ?)")
            .bind(&subject)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("update device_codes set state = 'approved', subject = ?, auth_time = ? where user_code = ?")
            .bind(&subject)
            .bind(now)
            .bind(user_code)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(Some(subject))
    }

    /// Deny the pending device code `user_code` is. The answer is
    /// whether there was such a code, pending and not expired.
    pub async fn deny(&self, user_code: &str) -> Result<bool, Error> {
        let now = crate::now();
        let denied = sqlx::query(
            "update device_codes set state = 'denied' where user_code = ? and state = 'pending' and expires > ?",
        )
        .bind(user_code)
        .bind(now)
        .execute(&self.store.pool)
        .await?
        .rows_affected();
        Ok(denied == 1)
    }
}
