//! What an issuer says of an account's user.

use reqwest::header::ACCEPT;
use serde_json::Value;

use super::issuer::issuer;
use super::{Account, Accounts, Error, Which};

impl Accounts {
    /// What `account`'s issuer says of its user, now: the JSON object
    /// the issuer's own endpoint answers — see
    /// [`Issuer::info_endpoint`](super::Issuer) — as the issuer wrote
    /// it. The rules:
    ///
    /// 1. **The issuer's own shape.** GitHub answers a user — its
    ///    `login`, `name`, `avatar_url` and the rest; Google answers
    ///    OpenID Connect's claims — `sub`, `email`, `name`,
    ///    `picture`. Nothing is renamed, and nothing is promised of
    ///    either beyond what its issuer does.
    /// 2. **Asked every time.** Nothing of the answer is kept: a name,
    ///    an address and a picture change, and what is answered is
    ///    what they are now.
    /// 3. **As the account.** The request carries the account's access
    ///    token, had as every token is — see
    ///    [`token`](Self::token): under the account's lock, renewed
    ///    first where it has expired, and the lock let go before the
    ///    issuer is asked. An issuer that refuses it, `401` or `403`,
    ///    is [`Error::SignedOut`].
    /// 4. **No token** is in the answer: the issuers' endpoints answer
    ///    none.
    pub async fn info(&self, account: &Account) -> Result<Value, Error> {
        let issuer = issuer(&account.issuer)?;
        // Rule 3.
        let token = self.token(account, Which::Access).await?;
        let response = self.http.get(issuer.info_endpoint).bearer_auth(token).header(ACCEPT, "application/json").send().await.map_err(Error::Http)?;
        let status = response.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(Error::SignedOut(account.clone()));
        }
        let body = response.bytes().await.map_err(Error::Http)?;
        let unread = || Error::Answer { url: issuer.info_endpoint.to_string(), status: status.as_u16(), body: String::from_utf8_lossy(&body).into_owned() };
        if !status.is_success() {
            return Err(unread());
        }
        // Rule 1.
        serde_json::from_slice(&body).map_err(|_| unread())
    }
}
