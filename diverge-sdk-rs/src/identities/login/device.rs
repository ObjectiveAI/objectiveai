//! Asking for a device code.

use serde::Deserialize;
use zeroize::Zeroizing;

use super::Pending;
use super::discovery::discover;
use crate::identities::claims::now;
use crate::identities::oauth::{Answer, post};
use crate::identities::{Error, Identities};

/// The interval RFC 8628 §3.2 has a client poll at when the issuer
/// names none.
const INTERVAL: u64 = 5;

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
    /// Begin signing in at `issuer` as the public client `client_id`,
    /// asking for `scope` — `openid` and `offline_access` added where
    /// it lacks them — and, where given, for an access token for
    /// `resource`. The answer carries the user code a person enters at
    /// the verification URI; [`Pending::wait`] ends the login. See the
    /// [rules](crate::identities), rule 6.
    pub async fn login(&self, issuer: &str, client_id: &str, scope: &str, resource: Option<&str>) -> Result<Pending, Error> {
        let (discovery, device) = discover(&self.http, issuer).await?;
        let mut values: Vec<&str> = scope.split_ascii_whitespace().collect();
        for needed in ["openid", "offline_access"] {
            if !values.contains(&needed) {
                values.push(needed);
            }
        }
        let scope = values.join(" ");
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
            issuer: discovery.issuer,
            client_id: client_id.to_string(),
            token_endpoint: discovery.token_endpoint,
            scope,
            resource: resource.map(str::to_string),
            device_code: Zeroizing::new(code.device_code),
            interval: code.interval.unwrap_or(INTERVAL),
            expires: now().saturating_add(i64::try_from(code.expires_in).unwrap_or(i64::MAX)),
        })
    }
}
