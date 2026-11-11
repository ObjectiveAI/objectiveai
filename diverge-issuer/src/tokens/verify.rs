//! Verifying an access token.

use jsonwebtoken::{Algorithm, Validation};

use super::{Access, TYP};
use crate::config::Config;
use crate::signing::Signing;

/// The claims of `token`, if it is an access token this issuer signed
/// and it is good now. The rules, as RFC 9068 §4 states them:
///
/// 1. The header's `typ` is `at+jwt`, and its `alg` is `RS256`.
/// 2. Its `kid` names the key the issuer read at its start, and the signature is that
///    key's.
/// 3. `iss` is this issuer, exactly, and `exp` has not passed, give or
///    take a minute of clock skew.
///
/// The audience is not checked: the one caller is the user info
/// endpoint, which serves a token for any audience.
pub fn verify(config: &Config, signing: &Signing, token: &str) -> Option<Access> {
    let header = jsonwebtoken::decode_header(token).ok()?;
    if header.typ.as_deref() != Some(TYP) || header.alg != Algorithm::RS256 {
        return None;
    }
    let key = signing.find(header.kid.as_deref()?)?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[config.issuer()]);
    validation.set_required_spec_claims(&["exp", "iss", "sub"]);
    validation.validate_aud = false;
    let data = jsonwebtoken::decode::<Access>(token, &key.decoding, &validation).ok()?;
    Some(data.claims)
}
