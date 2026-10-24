//! What a JWT says, read without its signature checked.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::alphabet::URL_SAFE;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use serde::Deserialize;

/// base64url, with or without the padding JWT leaves off.
const BASE64URL: GeneralPurpose =
    GeneralPurpose::new(&URL_SAFE, GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent));

/// The two claims read: the subject and the expiry.
#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct Claims {
    /// `sub`.
    pub(super) sub: Option<String>,
    /// `exp`, in seconds since the Unix epoch.
    pub(super) exp: Option<i64>,
}

/// The claims of `jwt`, its payload read and nothing checked; `None`
/// when it is no JWT. Only a token taken from the issuer's token
/// endpoint itself is read so, where nothing stands between the two
/// but TLS.
pub(super) fn claims(jwt: &str) -> Option<Claims> {
    let payload = jwt.split('.').nth(1)?;
    let bytes = BASE64URL.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Now, in whole seconds since the Unix epoch.
pub(super) fn now() -> i64 {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or(0);
    i64::try_from(seconds).unwrap_or(i64::MAX)
}
