//! Recording a signing key.

use super::{Keys, Record};
use crate::store::Error;

impl Keys<'_> {
    /// Record `record`, unretired.
    pub async fn add(&self, record: &Record) -> Result<(), Error> {
        sqlx::query("insert into keys (kid, entry, made) values (?, ?, ?)")
            .bind(&record.kid)
            .bind(&record.entry)
            .bind(record.made)
            .execute(&self.store.pool)
            .await?;
        Ok(())
    }
}
