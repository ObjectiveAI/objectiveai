//! A client's poll of a device code.

use super::{Approved, Devices};
use crate::store::Error;

/// The seconds a poll too soon adds to the code's interval, as RFC
/// 8628 §3.5 requires.
const SLOWER: i64 = 5;

/// What a poll finds — see the [rules](super).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Poll {
    /// No such code for this client.
    Unknown,
    /// The code has expired, and is removed.
    Expired,
    /// No one has decided yet.
    Pending,
    /// The poll came too soon; the interval is now longer.
    SlowDown,
    /// A person denied it, and it is removed.
    Denied,
    /// A person approved it, and it is removed: what the tokens say.
    Approved(Approved),
}

/// A device code's row, as a poll reads it.
type Row = (String, String, String, i64, i64, Option<i64>, String, Option<String>, Option<i64>);

impl Devices<'_> {
    /// Poll the device code whose hash is `hash`, as `client`, now.
    pub async fn poll(&self, hash: &str, client: &str) -> Result<Poll, Error> {
        let now = crate::now();
        let mut transaction = self.store.write().await?;
        let row: Option<Row> = sqlx::query_as(
            "select client, scope, audience, expires, interval, last_poll, state, subject, auth_time
                from device_codes where hash = ?",
        )
        .bind(hash)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some((owner, scope, audience, expires, interval, last_poll, state, subject, auth_time)) = row else {
            return Ok(Poll::Unknown);
        };
        if owner != client {
            return Ok(Poll::Unknown);
        }
        let remove = || sqlx::query("delete from device_codes where hash = ?").bind(hash);
        let poll = if expires <= now {
            remove().execute(&mut *transaction).await?;
            Poll::Expired
        } else {
            match (state.as_str(), subject, auth_time) {
                ("approved", Some(subject), Some(auth_time)) => {
                    remove().execute(&mut *transaction).await?;
                    Poll::Approved(Approved { subject, scope, audience, auth_time })
                }
                ("denied", _, _) => {
                    remove().execute(&mut *transaction).await?;
                    Poll::Denied
                }
                _ if last_poll.is_some_and(|last| now - last < interval) => {
                    sqlx::query("update device_codes set interval = interval + ?, last_poll = ? where hash = ?")
                        .bind(SLOWER)
                        .bind(now)
                        .bind(hash)
                        .execute(&mut *transaction)
                        .await?;
                    Poll::SlowDown
                }
                _ => {
                    sqlx::query("update device_codes set last_poll = ? where hash = ?")
                        .bind(now)
                        .bind(hash)
                        .execute(&mut *transaction)
                        .await?;
                    Poll::Pending
                }
            }
        };
        transaction.commit().await?;
        Ok(poll)
    }
}
