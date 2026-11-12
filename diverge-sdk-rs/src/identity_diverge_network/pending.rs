//! A login waiting for a person.

use std::time::Duration;

use serde::Deserialize;

use super::device::{Answer, post};
use super::login::now;
use super::record::{Record, write};
use super::who::who;
use super::{Account, Accounts, Error, Issuer, Tokens};
use crate::file_lock;

/// The seconds a `slow_down` adds to the interval, as RFC 8628 §3.5
/// requires.
const SLOWER: u64 = 5;

/// The grant a device code is spent by, RFC 8628 §3.4.
const GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// A token endpoint's answer, RFC 6749 §5.1.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct Answered {
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
    /// Where the person enters it: the issuer's own page.
    pub verification_uri: String,
    /// Where the person need not enter it: the page with the code in
    /// it, where the issuer gives one.
    pub verification_uri_complete: Option<String>,
    /// The seconds the code is good for.
    pub expires_in: u64,
    pub(super) accounts: Accounts,
    pub(super) issuer: &'static Issuer,
    pub(super) device_code: String,
    pub(super) interval: u64,
    pub(super) expires: i64,
}

impl Pending {
    /// Poll the token endpoint until the person decides, as RFC 8628
    /// §3.5 states: every `interval` seconds, five more at each
    /// `slow_down`, on through `authorization_pending`. An approval
    /// has the issuer say who signed in — see the [rules](super), rule
    /// 3 — and writes that account's file, replacing one already
    /// there; the answer is the account. A denial is
    /// [`Error::Denied`], an expired code [`Error::Expired`], any
    /// other refusal [`Error::Refused`], and none of them writes
    /// anything.
    pub async fn wait(self) -> Result<Account, Error> {
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
            match post::<Answered>(&self.accounts.http, self.issuer.token_endpoint, &form).await? {
                Answer::Answered(answered) => return self.keep(answered).await,
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

    /// The account `answered` is, written under its lock — see the
    /// [rules](super), rule 6: taken here, once the account is known,
    /// and let go once the file is written.
    async fn keep(&self, answered: Answered) -> Result<Account, Error> {
        let id = who(&self.accounts.http, self.issuer.who, &answered).await?;
        let account = Account { issuer: self.issuer.name.to_string(), id: id.clone() };
        let record = Record {
            issuer: account.issuer.clone(),
            id,
            tokens: Tokens {
                access_token: answered.access_token,
                access_expires: answered.expires_in.map(|seconds| now().saturating_add(i64::try_from(seconds).unwrap_or(i64::MAX))),
                refresh_token: answered.refresh_token,
                id_token: answered.id_token,
            },
        };
        let paths = self.accounts.paths(&account);
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        write(&paths, &record).await?;
        drop(held);
        Ok(account)
    }
}
