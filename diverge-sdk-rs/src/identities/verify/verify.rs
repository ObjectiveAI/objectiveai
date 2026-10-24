//! Verifying a token.

use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use super::{KeySets, Verified, VerifyError};
use crate::identities::claims::claims;

/// The algorithms a key set can check — see the [rules](super), rule
/// 2.
const ASYMMETRIC: [Algorithm; 9] = [
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
    Algorithm::ES256,
    Algorithm::ES384,
    Algorithm::EdDSA,
];

/// The leeway, in seconds, `exp` is checked with.
const LEEWAY: u64 = 60;

/// The claims a verified token is answered by.
#[derive(Debug, Deserialize)]
struct Checked {
    iss: String,
    sub: String,
    exp: i64,
}

impl KeySets {
    /// `token`, judged — see the [rules](super): its issuer accepted by
    /// `trusted`, its signature that issuer's, it unexpired, and its
    /// `aud` holding `audience` where one is given. The answer is who
    /// it names.
    pub async fn verify(
        &self,
        token: &str,
        trusted: impl Fn(&str) -> bool,
        audience: Option<&str>,
    ) -> Result<Verified, VerifyError> {
        let issuer = claims(token).and_then(|claims| claims.iss).ok_or(VerifyError::Malformed)?;
        if !trusted(&issuer) {
            return Err(VerifyError::Untrusted(issuer));
        }
        let header = jsonwebtoken::decode_header(token).map_err(|_| VerifyError::Malformed)?;
        if !ASYMMETRIC.contains(&header.alg) {
            return Err(VerifyError::Algorithm(format!("{:?}", header.alg)));
        }
        let kid = header.kid.ok_or(VerifyError::Malformed)?;
        let keys = self.keys(&issuer, &kid).await?;
        let jwk = keys.find(&kid).ok_or_else(|| VerifyError::UnknownKey { issuer: issuer.clone(), kid: kid.clone() })?;
        let key = DecodingKey::from_jwk(jwk).map_err(VerifyError::Invalid)?;
        let mut validation = Validation::new(header.alg);
        validation.leeway = LEEWAY;
        validation.set_issuer(&[&issuer]);
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);
        match audience {
            Some(audience) => validation.set_audience(&[audience]),
            None => validation.validate_aud = false,
        }
        let checked = jsonwebtoken::decode::<Checked>(token, &key, &validation).map_err(VerifyError::Invalid)?.claims;
        Ok(Verified { issuer: checked.iss, subject: checked.sub, expires: checked.exp })
    }
}
