//! The proof a token request carries that its sender holds a key.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore as _;

use super::Key;
use super::claims::now;
use crate::wire::handshake::Signer as _;

/// The header a proof is carried in, RFC 9449 §4.1.
pub(super) const HEADER: &str = "DPoP";

/// The header an issuer answers the nonce it wants in, RFC 9449 §8.
pub(super) const NONCE: &str = "DPoP-Nonce";

/// The algorithm every proof here is signed with, as an issuer names
/// it in `dpop_signing_alg_values_supported`: ECDSA over P-256 with
/// SHA-256.
pub(super) const ALG: &str = "ES256";

/// A proof of possession for one `POST` to `url`, RFC 9449 §4.2 —
/// DPoP — signed with `key`. The rules:
///
/// 1. **The header.** `typ` is `dpop+jwt`, `alg` is [`ALG`], and
///    `jwk` is the key's public half — see [`Key::jwk`].
/// 2. **The claims.** `jti`, sixteen random bytes as unpadded
///    base64url, new for every proof; `htm`, `POST`; `htu`, `url` as
///    it is requested, which carries no query and no fragment; `iat`,
///    now; and `nonce`, the one the issuer asked for, where it asked
///    for one.
/// 3. **The signature.** A JWS in the compact form, RFC 7515 §3.1:
///    the header and the claims, each as unpadded base64url, joined by
///    a `.`, signed by the key, the signature — the 32 bytes of `r`
///    then the 32 bytes of `s`, as RFC 7518 §3.4 lays an ES256 one out
///    — as unpadded base64url after another `.`.
/// 4. **One request each.** A proof is made for one request and sent
///    with it; a request made again is made with a new proof.
pub(super) fn proof(key: &Key, url: &str, nonce: Option<&str>) -> String {
    let mut jti = [0u8; 16];
    rand::rng().fill_bytes(&mut jti);
    let header = format!(r#"{{"typ":"dpop+jwt","alg":"{ALG}","jwk":{}}}"#, key.jwk());
    let mut claims = serde_json::json!({
        "jti": URL_SAFE_NO_PAD.encode(jti),
        "htm": "POST",
        "htu": url,
        "iat": now(),
    });
    if let Some(nonce) = nonce {
        claims["nonce"] = serde_json::Value::String(nonce.to_string());
    }
    let signed = format!("{}.{}", URL_SAFE_NO_PAD.encode(header), URL_SAFE_NO_PAD.encode(claims.to_string()));
    let signature = key.sign(signed.as_bytes());
    format!("{signed}.{}", URL_SAFE_NO_PAD.encode(signature))
}
