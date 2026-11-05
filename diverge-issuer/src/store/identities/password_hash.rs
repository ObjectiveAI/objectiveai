//! Reading an identity's password's hash.

use super::Identities;
use crate::store::Error;

impl Identities<'_> {
    /// The hash of `subject`'s password, or `None` when there is no
    /// such identity: what a sign-in is checked against.
    pub async fn password_hash(&self, subject: &str) -> Result<Option<String>, Error> {
        let hash = sqlx::query_scalar("select password_hash from identities where subject = ?")
            .bind(subject)
            .fetch_optional(&self.store.pool)
            .await?;
        Ok(hash)
    }
}
