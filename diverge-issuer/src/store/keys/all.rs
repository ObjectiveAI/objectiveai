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

    /// Record `record` and remove every other key, and, with
    /// `sign_out`, revoke every refresh chain: one transaction, whole or
    /// not at all. The answer is how many keys were removed and how
    /// many chains revoked.
    pub async fn replace(&self, record: &Record, sign_out: bool) -> Result<(u64, u64), Error> {
        let mut transaction = self.store.write().await?;
        sqlx::query("insert into keys (kid, pem, made) values (?, ?, ?)")
            .bind(&record.kid)
            .bind(&record.pem)
            .bind(record.made)
            .execute(&mut *transaction)
            .await?;
        let removed = sqlx::query("delete from keys where kid <> ?")
            .bind(&record.kid)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        let signed_out = if sign_out {
            sqlx::query("update refresh_chains set revoked = ? where revoked is null")
                .bind(crate::now())
                .execute(&mut *transaction)
                .await?
                .rows_affected()
        } else {
            0
        };
        transaction.commit().await?;
        Ok((removed, signed_out))
    }
}
