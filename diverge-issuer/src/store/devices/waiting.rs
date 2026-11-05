//! The device codes waiting for the operator.

use super::{Devices, Waiting};
use crate::store::Error;

impl Devices<'_> {
    /// Every pending device code that has not expired, the soonest to
    /// expire first.
    pub async fn waiting(&self) -> Result<Vec<Waiting>, Error> {
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            "select user_code, login_hint, expires from device_codes
                where state = 'pending' and expires > ? order by expires, user_code",
        )
        .bind(crate::now())
        .fetch_all(&self.store.pool)
        .await?;
        Ok(rows.into_iter().map(|(user_code, subject, expires)| Waiting { user_code, subject, expires }).collect())
    }
}
