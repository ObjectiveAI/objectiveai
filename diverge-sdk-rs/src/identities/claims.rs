//! What a JWT says, read without its signature checked.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::alphabet::URL_SAFE;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use serde::Deserialize;

use crate::wire::frame::auth::THUMBPRINT_LEN;

/// base64url, with or without the padding JWT leaves off.
const BASE64URL: GeneralPurpose =
    GeneralPurpose::new(&URL_SAFE, GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent));

/// RFC 7800's `cnf`, as far as it is read: the thumbprint of the key
/// a token is bound to, RFC 9449 §6.1.
#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct Confirmation {
    /// `jkt`: the RFC 7638 SHA-256 thumbprint of the key the token's
    /// presenter proves it holds, as unpadded base64url.
    jkt: Option<String>,
}

impl Confirmation {
    /// The thumbprint the claim names: its `jkt`, where that is
    /// thirty-two bytes as base64url. `None` for any other.
    pub(super) fn thumbprint(&self) -> Option<[u8; THUMBPRINT_LEN]> {
        BASE64URL.decode(self.jkt.as_deref()?).ok()?.as_slice().try_into().ok()
    }
}

/// The claims read: the issuer, the subject, the expiry, and the key
/// the token names.
#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct Claims {
    /// `iss`: which issuer's keys a token is checked against, read
    /// before the check.
    pub(super) iss: Option<String>,
    /// `sub`.
    pub(super) sub: Option<String>,
    /// `exp`, in seconds since the Unix epoch.
    pub(super) exp: Option<i64>,
    /// `cnf`: the key the token is bound to, by its thumbprint.
    pub(super) cnf: Option<Confirmation>,
}

/// The claims of `jwt`, its payload read and nothing checked; `None`
/// when it is no JWT. A token taken from the issuer's token endpoint
/// itself is read so, where nothing stands between the two but TLS;
/// any other token only for its `iss`, to choose the keys its
/// signature is then checked against.
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
