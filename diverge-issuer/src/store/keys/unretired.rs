//! The signing keys in use.

use super::{Keys, Record};
use crate::store::Error;

impl Keys<'_> {
    /// Every record not retired, the newest last.
    pub async fn unretired(&self) -> Result<Vec<Record>, Error> {
        let rows: Vec<(String, String, i64)> =
            sqlx::query_as("select kid, entry, made from keys where retired is null order by made, kid")
                .fetch_all(&self.store.pool)
                .await?;
        Ok(rows.into_iter().map(|(kid, entry, made)| Record { kid, entry, made }).collect())
    }
}
