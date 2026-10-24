//! Retiring signing keys.

use super::Keys;
use crate::store::Error;

impl Keys<'_> {
    /// Retire, as of now, every unretired key but `kid`. The answer is
    /// how many were retired.
    pub async fn retire_all_but(&self, kid: &str) -> Result<u64, Error> {
        let retired = sqlx::query("update keys set retired = ? where retired is null and kid <> ?")
            .bind(crate::now())
            .bind(kid)
            .execute(&self.store.pool)
            .await?
            .rows_affected();
        Ok(retired)
    }
}
