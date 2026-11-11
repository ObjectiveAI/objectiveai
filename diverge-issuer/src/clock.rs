//! The time, as the issuer records it.

use std::time::{SystemTime, UNIX_EPOCH};

/// Now, in whole seconds since the Unix epoch: the one unit every
/// time in `issuer.sqlite` and every time in a token is written in. A
/// clock set before the epoch reads `0`.
pub fn now() -> i64 {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or(0);
    i64::try_from(seconds).unwrap_or(i64::MAX)
}
