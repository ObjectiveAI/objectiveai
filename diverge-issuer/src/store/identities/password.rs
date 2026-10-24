//! Changing an identity's password.

use super::Identities;
use crate::store::Error;

impl Identities<'_> {
    /// Change `subject`'s password to the one `password_hash` is the
    /// hash of, and revoke every refresh token issued to it, so a
    /// session begun with the old password does not outlive it. The
    /// answer is whether there was such an identity.
    pub async fn password(&self, subject: &str, password_hash: &str) -> Result<bool, Error> {
        let mut transaction = self.store.pool.begin().await?;
        let changed = sqlx::query("update identities set password_hash = ? where subject = ?")
            .bind(password_hash)
            .bind(subject)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        sqlx::query("update refresh_chains set revoked = ? where subject = ? and revoked is null")
            .bind(crate::now())
            .bind(subject)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(changed == 1)
    }
}
