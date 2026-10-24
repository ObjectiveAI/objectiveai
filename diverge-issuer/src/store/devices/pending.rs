//! A pending device code, by its user code.

use super::{Devices, Pending};
use crate::store::Error;

impl Devices<'_> {
    /// The pending device code `user_code` is, if it has not expired.
    pub async fn pending(&self, user_code: &str) -> Result<Option<Pending>, Error> {
        let row: Option<(String,)> = sqlx::query_as(
            "select scope from device_codes where user_code = ? and state = 'pending' and expires > ?",
        )
        .bind(user_code)
        .bind(crate::now())
        .fetch_optional(&self.store.pool)
        .await?;
        Ok(row.map(|(scope,)| Pending { scope }))
    }
}
