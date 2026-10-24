//! Every signing key, as a listing reports it.

use super::{Keys, Listed};
use crate::store::Error;

impl Keys<'_> {
    /// Every key, retired or not, the oldest first, never the key
    /// itself.
    pub async fn list(&self) -> Result<Vec<Listed>, Error> {
        let rows: Vec<(String, i64, Option<i64>)> = sqlx::query_as("select kid, made, retired from keys order by made, kid")
            .fetch_all(&self.store.pool)
            .await?;
        Ok(rows.into_iter().map(|(kid, made, retired)| Listed { kid, made, retired }).collect())
    }
}
