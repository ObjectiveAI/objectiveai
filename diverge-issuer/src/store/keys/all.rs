//! The signing keys.

use super::{Keys, Record};
use crate::store::Error;

impl Keys<'_> {
    /// Every key, the newest last: the one key there is, but between a
    /// rotation's mint and its replacing.
    pub async fn all(&self) -> Result<Vec<Record>, Error> {
        let rows: Vec<(String, String, i64)> = sqlx::query_as("select kid, pem, made from keys order by made, kid")
            .fetch_all(&self.store.pool)
            .await?;
        Ok(rows.into_iter().map(|(kid, pem, made)| Record { kid, pem, made }).collect())
    }

    /// Remove every key but `kid`: what a rotation does once the new
    /// key is recorded. The answer is how many were removed.
    pub async fn remove_all_but(&self, kid: &str) -> Result<u64, Error> {
        let removed = sqlx::query("delete from keys where kid <> ?")
            .bind(kid)
            .execute(&self.store.pool)
            .await?
            .rows_affected();
        Ok(removed)
    }
}
