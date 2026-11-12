//! Asking a provider for a device code.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::device::{Answer, post};
use super::provider::provider;
use super::{Accounts, Error, Pending};

/// The interval RFC 8628 §3.2 has a client poll at when the provider
/// names none.
const INTERVAL: u64 = 5;

/// The device authorization endpoint's answer, RFC 8628 §3.2 — and
/// Google's, which names the page `verification_url`.
#[derive(Debug, Clone, Deserialize)]
struct DeviceCode {
    device_code: String,
    user_code: String,
    #[serde(alias = "verification_url")]
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: Option<u64>,
}

/// Now, in seconds since the Unix epoch.
pub(super) fn now() -> i64 {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or(0);
    i64::try_from(seconds).unwrap_or(i64::MAX)
}

impl Accounts {
    /// Begin signing in at the provider named `provider` — see the
    /// [rules](super), rule 2: a device code asked for, as the client
    /// `identity.diverge.network` is there, with the provider's scope.
    /// The answer carries the user code a person enters at the
    /// provider's page; [`Pending::wait`] ends the login. A name that
    /// is no provider's is [`Error::Provider`], and nothing is asked
    /// of anybody.
    pub async fn login(&self, provider: &str) -> Result<Pending, Error> {
        let provider = self::provider(provider)?;
        let form = [("client_id", provider.client_id), ("scope", provider.scope)];
        let code = match post::<DeviceCode>(&self.http, provider.device_endpoint, &form).await? {
            Answer::Answered(code) => code,
            Answer::Refused { code, description } => return Err(Error::Refused { code, description }),
        };
        Ok(Pending {
            user_code: code.user_code,
            verification_uri: code.verification_uri,
            verification_uri_complete: code.verification_uri_complete,
            expires_in: code.expires_in,
            accounts: self.clone(),
            provider,
            device_code: code.device_code,
            interval: code.interval.unwrap_or(INTERVAL),
            expires: now().saturating_add(i64::try_from(code.expires_in).unwrap_or(i64::MAX)),
        })
    }
}
