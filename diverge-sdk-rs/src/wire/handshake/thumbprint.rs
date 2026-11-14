//! A key's thumbprint.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest as _, Sha256};

use crate::wire::frame::auth::{POINT_LEN, THUMBPRINT_LEN};

/// The RFC 7638 SHA-256 thumbprint of the P-256 public key `key`: the
/// SHA-256 of the key as a JSON Web Key with its required members
/// alone, in lexicographic order and with nothing between them —
///
/// ```text
/// {"crv":"P-256","kty":"EC","x":"<x>","y":"<y>"}
/// ```
///
/// — `x` and `y` the 32 bytes of each coordinate as unpadded
/// base64url. It is what a token bound to the key carries as
/// `cnf.jkt`, there as unpadded base64url, RFC 9449 §6.1; an end
/// computes it over the key the other end sent and holds it against
/// the one the other end's token names.
pub fn thumbprint(key: &[u8; POINT_LEN]) -> [u8; THUMBPRINT_LEN] {
    let (x, y) = key[1..].split_at(32);
    let canonical = format!(r#"{{"crv":"P-256","kty":"EC","x":"{}","y":"{}"}}"#, URL_SAFE_NO_PAD.encode(x), URL_SAFE_NO_PAD.encode(y));
    Sha256::digest(canonical.as_bytes()).into()
}
