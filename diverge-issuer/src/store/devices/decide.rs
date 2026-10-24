//! A person's decision on a device code.

use super::Devices;
use crate::store::Error;

impl Devices<'_> {
    /// Decide the pending device code `user_code` is: approved for
    /// `subject`, who signed in now, or, with no subject, denied. The
    /// answer is whether there was such a code, pending and not
    /// expired.
    pub async fn decide(&self, user_code: &str, subject: Option<&str>) -> Result<bool, Error> {
        let now = crate::now();
        let state = if subject.is_some() { "approved" } else { "denied" };
        let decided = sqlx::query(
            "update device_codes set state = ?, subject = ?, auth_time = ?
                where user_code = ? and state = 'pending' and expires > ?",
        )
        .bind(state)
        .bind(subject)
        .bind(subject.map(|_| now))
        .bind(user_code)
        .bind(now)
        .execute(&self.store.pool)
        .await?
        .rows_affected();
        Ok(decided == 1)
    }
}
