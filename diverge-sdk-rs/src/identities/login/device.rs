//! Asking for a device code.

use serde::Deserialize;
use zeroize::Zeroizing;

use super::Pending;
use crate::file_lock;
use crate::identities::claims::now;
use crate::identities::discovery::{at, discover};
use crate::identities::identities::folder_owned;
use crate::identities::key;
use crate::identities::oauth::{Answer, post};
use crate::identities::{Error, Identities, Name};

/// The interval RFC 8628 §3.2 has a client poll at when the issuer
/// names none.
const INTERVAL: u64 = 5;

/// `scope` with `openid` and `offline_access` added where it lacks
/// them: the ID token names the subject, and `offline_access` is how
/// most issuers are asked for a refresh token.
pub(super) fn scoped(scope: &str) -> String {
    let mut values: Vec<&str> = scope.split_ascii_whitespace().collect();
    for needed in ["openid", "offline_access"] {
        if !values.contains(&needed) {
            values.push(needed);
        }
    }
    values.join(" ")
}

/// The device authorization endpoint's answer, RFC 8628 §3.2.
#[derive(Debug, Clone, Deserialize)]
struct DeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: Option<u64>,
}

impl Identities {
    /// Begin signing `name` in as the public client `client_id`,
    /// asking for `scope` — `openid` and `offline_access` added where
    /// it lacks them — and, where given, for an access token for
    /// `resource`. The identity's lock is taken first, waiting while
    /// another holds it, and held by the [`Pending`] until the login
    /// ends, however it ends: a failure here lets it go at once. The
    /// answer carries the user code a person enters at the
    /// verification URI; [`Pending::wait`] ends the login. See the
    /// [rules](crate::identities), rule 6.
    pub async fn login(&self, name: &Name, client_id: &str, scope: &str, resource: Option<&str>) -> Result<Pending, Error> {
        let paths = self.paths(name);
        folder_owned(&paths.folder).await?;
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        self.begin(held, name, client_id, scope, resource).await
    }

    /// [`login`](Self::login), the lock taken only if nobody holds it:
    /// [`Error::Held`] at once if another login or a reader of the
    /// identity holds it, and nothing asked of the issuer.
    pub async fn try_login(&self, name: &Name, client_id: &str, scope: &str, resource: Option<&str>) -> Result<Pending, Error> {
        let paths = self.paths(name);
        folder_owned(&paths.folder).await?;
        let Some(held) = file_lock::try_exclusive(&paths.lock).await.map_err(Error::Lock)? else {
            return Err(Error::Held(name.clone()));
        };
        self.begin(held, name, client_id, scope, resource).await
    }

    /// The login, the identity's lock in hand: the identity's key
    /// read, and made where it has none; the discovery document read;
    /// the device code asked for; and the [`Pending`] holding the lock
    /// from here on — a failure here lets it go at once.
    async fn begin(&self, held: file_lock::Held, name: &Name, client_id: &str, scope: &str, resource: Option<&str>) -> Result<Pending, Error> {
        let issuer = name.issuer.as_str();
        let jwk = key::ensure(&self.paths(name)).await?.jwk();
        let discovery = discover(&self.http, issuer).await?;
        let device = discovery
            .device_authorization_endpoint
            .clone()
            .ok_or_else(|| Error::Discovery(format!("the document at `{}` names no device authorization endpoint", at(issuer))))?;
        let scope = scoped(scope);
        let mut form = vec![("client_id", client_id), ("scope", scope.as_str())];
        if let Some(resource) = resource {
            form.push(("resource", resource));
        }
        let code = match post::<DeviceCode>(&self.http, &device, &form).await? {
            Answer::Answered(code) => code,
            Answer::Refused(refusal) => return Err(Error::Refused { code: refusal.error, description: refusal.error_description }),
        };
        Ok(Pending {
            user_code: code.user_code,
            verification_uri: code.verification_uri,
            verification_uri_complete: code.verification_uri_complete,
            expires_in: code.expires_in,
            identities: self.clone(),
            name: name.clone(),
            _held: held,
            issuer: discovery.issuer,
            client_id: client_id.to_string(),
            token_endpoint: discovery.token_endpoint,
            scope,
            resource: resource.map(str::to_string),
            jwk,
            device_code: Zeroizing::new(code.device_code),
            interval: code.interval.unwrap_or(INTERVAL),
            expires: now().saturating_add(i64::try_from(code.expires_in).unwrap_or(i64::MAX)),
        })
    }
}
