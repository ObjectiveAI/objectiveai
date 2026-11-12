//! A login waiting for a person.

use std::time::Duration;

use serde::Deserialize;

use super::device::{Answer, post};
use super::login::now;
use super::record::Record;
use super::who::who;
use super::{Account, Accounts, Error, Provider};

/// The seconds a `slow_down` adds to the interval, as RFC 8628 §3.5
/// requires.
const SLOWER: u64 = 5;

/// The grant a device code is spent by, RFC 8628 §3.4.
const GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// A token endpoint's answer, RFC 6749 §5.1.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct Tokens {
    /// The access token.
    pub(super) access_token: Option<String>,
    /// The seconds it is good for, where it expires.
    pub(super) expires_in: Option<u64>,
    /// A refresh token.
    pub(super) refresh_token: Option<String>,
    /// The ID token.
    pub(super) id_token: Option<String>,
}

/// A login begun: what to show the person, and the device code the
/// wait polls with, which is never shown.
pub struct Pending {
    /// The code the person enters.
    pub user_code: String,
    /// Where the person enters it: the provider's own page.
    pub verification_uri: String,
    /// Where the person need not enter it: the page with the code in
    /// it, where the provider gives one.
    pub verification_uri_complete: Option<String>,
    /// The seconds the code is good for.
    pub expires_in: u64,
    pub(super) accounts: Accounts,
    pub(super) provider: &'static Provider,
    pub(super) device_code: String,
    pub(super) interval: u64,
    pub(super) expires: i64,
}

impl Pending {
    /// Poll the token endpoint until the person decides, as RFC 8628
    /// §3.5 states: every `interval` seconds, five more at each
    /// `slow_down`, on through `authorization_pending`. An approval
    /// has the provider say who signed in — see the
    /// [rules](super), rule 3 — and writes that account's file,
    /// replacing one already there; the answer is the account. A
    /// denial is [`Error::Denied`], an expired code [`Error::Expired`],
    /// any other refusal [`Error::Refused`], and none of them writes
    /// anything.
    pub async fn wait(self) -> Result<Account, Error> {
        let mut interval = self.interval;
        loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;
            if now() >= self.expires {
                return Err(Error::Expired);
            }
            let mut form = vec![("grant_type", GRANT), ("device_code", self.device_code.as_str()), ("client_id", self.provider.client_id)];
            if let Some(secret) = self.provider.client_secret {
                form.push(("client_secret", secret));
            }
            match post::<Tokens>(&self.accounts.http, self.provider.token_endpoint, &form).await? {
                Answer::Answered(tokens) => return self.keep(tokens).await,
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

    /// The account `tokens` are, written.
    async fn keep(&self, tokens: Tokens) -> Result<Account, Error> {
        let id = who(&self.accounts.http, self.provider.who, &tokens).await?;
        let access_expires = tokens.expires_in.map(|seconds| now().saturating_add(i64::try_from(seconds).unwrap_or(i64::MAX)));
        let record = Record {
            provider: self.provider.name.to_string(),
            id: id.clone(),
            access_token: tokens.access_token,
            access_expires,
            refresh_token: tokens.refresh_token,
            id_token: tokens.id_token,
        };
        self.accounts.write(&record).await?;
        Ok(Account { provider: record.provider, id })
    }
}
