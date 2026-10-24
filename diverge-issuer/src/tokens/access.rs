//! The access token.

use jsonwebtoken::{Algorithm, Header};
use serde::{Deserialize, Serialize};

use super::{Error, mint};
use crate::config::Config;
use crate::signing::Signing;

/// The `typ` an access token's header carries, as RFC 9068 §2.1
/// requires, so that no other JWT is taken for one.
pub const TYP: &str = "at+jwt";

/// An access token's claims, as RFC 9068 §2.2 names them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Access {
    /// The issuer identifier.
    pub iss: String,
    /// The subject: the identity signed in, or for `client_credentials`
    /// the client itself.
    pub sub: String,
    /// The one audience the token is for: the server that admits it.
    pub aud: String,
    /// When it stops being good, in seconds since the Unix epoch.
    pub exp: i64,
    /// When it was issued.
    pub iat: i64,
    /// Its own id, unique among the tokens the issuer signs.
    pub jti: String,
    /// The client it was issued to.
    pub client_id: String,
    /// The scope granted, space-separated; absent when none was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// An access token, signed: the JWT, and the seconds it is good for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issued {
    /// The JWT.
    pub token: String,
    /// The seconds from now it is good for: `expires_in`.
    pub expires_in: u64,
}

/// Sign an access token for `sub` at `aud`, issued to `client_id`
/// with `scope`, good for `access_token_seconds`, with the key that
/// signs.
pub fn access(config: &Config, signing: &Signing, sub: &str, aud: &str, client_id: &str, scope: &str) -> Result<Issued, Error> {
    let now = crate::now();
    let lifetime = config.access_token_seconds;
    let claims = Access {
        iss: config.issuer(),
        sub: sub.to_string(),
        aud: aud.to_string(),
        exp: now.saturating_add(i64::try_from(lifetime).unwrap_or(i64::MAX)),
        iat: now,
        jti: mint().expose()[..32].to_string(),
        client_id: client_id.to_string(),
        scope: (!scope.is_empty()).then(|| scope.to_string()),
    };
    let key = signing.current();
    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some(TYP.to_string());
    header.kid = Some(key.kid.clone());
    let token = jsonwebtoken::encode(&header, &claims, &key.encoding).map_err(Error::Access)?;
    Ok(Issued { token, expires_in: lifetime })
}
