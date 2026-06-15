//! Time conversion helpers for CLI command responses.
//!
//! Timestamps are stored in postgres as `BIGINT` unix-seconds, but the
//! CLI surfaces them to callers as RFC3339 strings. These helpers do
//! that conversion at the response boundary. The format
//! (`chrono::to_rfc3339`, numeric `+00:00` offset — not a bare `Z`)
//! matches what `db::query` emits for `TIMESTAMPTZ` columns, so every
//! CLI-emitted timestamp is byte-uniform.

/// Render unix-seconds as an RFC3339 string.
///
/// `from_timestamp` only returns `None` for seconds far outside
/// chrono's representable range — impossible for real db unix
/// timestamps. We fall back to the unix epoch so the result is always
/// a valid RFC3339 string and this never panics.
pub fn unix_to_rfc3339(secs: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .unwrap_or_else(|| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
                .expect("unix epoch is in range")
        })
        .to_rfc3339()
}

/// `Option` counterpart of [`unix_to_rfc3339`] — `None` stays `None`.
pub fn unix_to_rfc3339_opt(secs: Option<i64>) -> Option<String> {
    secs.map(unix_to_rfc3339)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_renders_rfc3339() {
        assert_eq!(unix_to_rfc3339(0), "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn option_passthrough() {
        assert_eq!(unix_to_rfc3339_opt(None), None);
        assert_eq!(
            unix_to_rfc3339_opt(Some(0)),
            Some("1970-01-01T00:00:00+00:00".to_string())
        );
    }

    #[test]
    fn out_of_range_falls_back_to_epoch_not_panic() {
        // i64::MAX seconds is far outside chrono's range → fallback.
        assert_eq!(unix_to_rfc3339(i64::MAX), "1970-01-01T00:00:00+00:00");
    }
}
