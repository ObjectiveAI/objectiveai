//! Asking an issuer for a device code, and waiting on it.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::device::{Answer, post};
use super::issuer::issuer;
use super::pending::Answered;
use super::who::who;
use super::{Accounts, Error, Issuer, Pending, Tokens};

/// The interval RFC 8628 §3.2 has a client poll at when the issuer
/// names none.
const INTERVAL: u64 = 5;

/// The seconds a `slow_down` adds to the interval, as RFC 8628 §3.5
/// requires.
const SLOWER: u64 = 5;

/// The grant a device code is spent by, RFC 8628 §3.4.
const GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

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

/// A login begun at an issuer: what to show the person, and the
/// device code the wait polls with, which is never shown. It is tied
/// to no store: [`wait`](Self::wait) answers whose account the login
/// is and its tokens, for whoever began it to keep.
pub struct Device {
    /// The code the person enters.
    pub user_code: String,
    /// Where the person enters it: the issuer's own page.
    pub verification_uri: String,
    /// Where the person need not enter it: the page with the code in
    /// it, where the issuer gives one.
    pub verification_uri_complete: Option<String>,
    /// The seconds the code is good for.
    pub expires_in: u64,
    issuer: &'static Issuer,
    device_code: String,
    interval: u64,
    expires: i64,
}

/// Begin signing in at `issuer` — see the [rules](super), rule 2: a
/// device code asked for, as the client `identity.diverge.network` is
/// there, with the issuer's scope. The answer carries the user code a
/// person enters at the issuer's page; [`Device::wait`] ends the
/// login.
pub async fn begin(http: &reqwest::Client, issuer: &'static Issuer) -> Result<Device, Error> {
    let form = [("client_id", issuer.client_id), ("scope", issuer.scope)];
    let code = match post::<DeviceCode>(http, issuer.device_endpoint, &form).await? {
        Answer::Answered(code) => code,
        Answer::Refused { code, description } => return Err(Error::Refused { code, description }),
    };
    Ok(Device {
        user_code: code.user_code,
        verification_uri: code.verification_uri,
        verification_uri_complete: code.verification_uri_complete,
        expires_in: code.expires_in,
        issuer,
        device_code: code.device_code,
        interval: code.interval.unwrap_or(INTERVAL),
        expires: now().saturating_add(i64::try_from(code.expires_in).unwrap_or(i64::MAX)),
    })
}

impl Device {
    /// The issuer the login is at.
    pub fn issuer(&self) -> &'static Issuer {
        self.issuer
    }

    /// Poll the token endpoint until the person decides, as RFC 8628
    /// §3.5 states: every `interval` seconds, five more at each
    /// `slow_down`, on through `authorization_pending`. An approval
    /// has the issuer say who signed in — see the [rules](super), rule
    /// 3 — and the answer is that user's id at the issuer, and the
    /// tokens the account is. A denial is [`Error::Denied`], an
    /// expired code [`Error::Expired`], any other refusal
    /// [`Error::Refused`]. Nothing is written here.
    pub async fn wait(self, http: &reqwest::Client) -> Result<(String, Tokens), Error> {
        let mut interval = self.interval;
        loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;
            if now() >= self.expires {
                return Err(Error::Expired);
            }
            let mut form = vec![("grant_type", GRANT), ("device_code", self.device_code.as_str()), ("client_id", self.issuer.client_id)];
            if let Some(secret) = self.issuer.client_secret {
                form.push(("client_secret", secret));
            }
            match post::<Answered>(http, self.issuer.token_endpoint, &form).await? {
                Answer::Answered(answered) => {
                    let id = who(http, self.issuer.who, &answered).await?;
                    let tokens = Tokens {
                        access_token: answered.access_token,
                        access_expires: answered.expires_in.map(|seconds| now().saturating_add(i64::try_from(seconds).unwrap_or(i64::MAX))),
                        refresh_token: answered.refresh_token,
                        id_token: answered.id_token,
                    };
                    return Ok((id, tokens));
                }
                Answer::Refused { code, description } => match code.as_str() {
                    "authorization_pending" => {}
                    "slow_down" => interval = interval.saturating_add(SLOWER),
                    "expired_token" => return Err(Error::Expired),
                    "access_denied" => return Err(Error::Denied),
                    _ => return Err(Error::Refused { code, description }),
                },
            }
        }
    }
}

impl Accounts {
    /// Begin signing in at the issuer named `issuer` — see [`begin`] —
    /// for an account kept in a file here. The answer carries the user
    /// code a person enters at the issuer's page; [`Pending::wait`]
    /// ends the login. A name that is no issuer's is
    /// [`Error::Issuer`], and nothing is asked of anybody. No lock is
    /// taken yet: whose account it is, is not known until the login
    /// ends.
    pub async fn login(&self, issuer: &str) -> Result<Pending, Error> {
        let device = begin(&self.http, self::issuer(issuer)?).await?;
        Ok(Pending {
            user_code: device.user_code.clone(),
            verification_uri: device.verification_uri.clone(),
            verification_uri_complete: device.verification_uri_complete.clone(),
            expires_in: device.expires_in,
            accounts: self.clone(),
            device,
        })
    }
}
