//! Issuing a device code.

use super::{Devices, New};
use crate::store::{Error, names};

impl Devices<'_> {
    /// Keep `device`, pending, and remove every device code that has
    /// expired. The answer is `false`, and nothing is kept, when its
    /// user code is held by another code: the caller draws another.
    pub async fn add(&self, device: &New) -> Result<bool, Error> {
        let mut transaction = self.store.write().await?;
        sqlx::query("delete from device_codes where expires <= ?")
            .bind(crate::now())
            .execute(&mut *transaction)
            .await?;
        let added = sqlx::query(
            "insert into device_codes (hash, user_code, client, scope, audience, expires, interval, state)
                values (?, ?, ?, ?, ?, ?, ?, 'pending')",
        )
        .bind(&device.hash)
        .bind(&device.user_code)
        .bind(&device.client)
        .bind(&device.scope)
        .bind(&device.audience)
        .bind(device.expires)
        .bind(device.interval)
        .execute(&mut *transaction)
        .await;
        match added {
            Ok(_) => {}
            Err(error) if names::taken(&error) => return Ok(false),
            Err(error) => return Err(Error::Query(error)),
        }
        transaction.commit().await?;
        Ok(true)
    }
}
