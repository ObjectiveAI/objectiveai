//! Every identity.

use super::{Identities, Identity};
use crate::store::Error;

impl Identities<'_> {
    /// Every identity, by subject.
    pub async fn list(&self) -> Result<Vec<Identity>, Error> {
        let rows: Vec<(String, i64)> = sqlx::query_as("select subject, made from identities order by subject")
            .fetch_all(&self.store.pool)
            .await?;
        Ok(rows.into_iter().map(|(subject, made)| Identity { subject, made }).collect())
    }
}
