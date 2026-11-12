//! What a provider says of an account's user.

use reqwest::header::ACCEPT;
use serde_json::Value;

use super::account::usable;
use super::provider::provider;
use super::{Account, Accounts, Error};

impl Accounts {
    /// What `account`'s provider says of its user, now: the JSON
    /// object the provider's own endpoint answers — see
    /// [`Provider::info_endpoint`](super::Provider) — as the provider
    /// wrote it. The rules:
    ///
    /// 1. **The provider's own shape.** GitHub answers a user — its
    ///    `login`, `name`, `avatar_url` and the rest; Google answers
    ///    OpenID Connect's claims — `sub`, `email`, `name`,
    ///    `picture`. Nothing is renamed, and nothing is promised of
    ///    either beyond what its provider does.
    /// 2. **Asked every time.** Nothing of the answer is kept: a name,
    ///    an address and a picture change, and what is answered is
    ///    what they are now.
    /// 3. **As the account.** The request carries the account's access
    ///    token, renewed first where it has expired — see
    ///    [`access`](Self::access). A provider that refuses it, `401`
    ///    or `403`, is [`Error::SignedOut`].
    /// 4. An account whose provider is none, whose id is none, or
    ///    which has no file, is [`Error::Missing`] — or
    ///    [`Error::Provider`], for the provider.
    /// 5. **No token** is in the answer: the providers' endpoints
    ///    answer none.
    pub async fn info(&self, account: &Account) -> Result<Value, Error> {
        // Rule 4.
        let provider = provider(&account.provider)?;
        if !usable(&account.id) {
            return Err(Error::Missing(account.clone()));
        }
        let mut record = self.read(account).await?.ok_or_else(|| Error::Missing(account.clone()))?;
        // Rule 3.
        let token = self.access(provider, &mut record).await?;
        let response = self.http.get(provider.info_endpoint).bearer_auth(token).header(ACCEPT, "application/json").send().await.map_err(Error::Http)?;
        let status = response.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(Error::SignedOut(account.clone()));
        }
        let body = response.bytes().await.map_err(Error::Http)?;
        let unread = || Error::Answer { url: provider.info_endpoint.to_string(), status: status.as_u16(), body: String::from_utf8_lossy(&body).into_owned() };
        if !status.is_success() {
            return Err(unread());
        }
        // Rule 1.
        serde_json::from_slice(&body).map_err(|_| unread())
    }
}
