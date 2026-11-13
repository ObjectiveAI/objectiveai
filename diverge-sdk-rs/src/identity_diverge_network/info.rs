//! What an issuer says of an account's user.

use reqwest::header::ACCEPT;
use serde_json::Value;

use super::issuer::issuer;
use super::{Account, Accounts, Error, Issuer, Which};

/// What `issuer` says of the user `access_token` is, now: the JSON
/// object the issuer's own endpoint answers — see
/// [`Issuer::info_endpoint`](super::Issuer) — as the issuer wrote it.
/// The rules:
///
/// 1. **The issuer's own shape.** GitHub answers a user — its
///    `login`, `name`, `avatar_url` and the rest; Google answers
///    OpenID Connect's claims — `sub`, `email`, `name`, `picture`.
///    Nothing is renamed, and nothing is promised of either beyond
///    what its issuer does.
/// 2. **Asked every time.** Nothing of the answer is kept: a name, an
///    address and a picture change, and what is answered is what they
///    are now.
/// 3. **Refused.** An issuer that refuses the token, `401` or `403`,
///    is [`Error::Refused`], `unauthorized`: the account is signed out
///    there.
/// 4. **No token** is in the answer: the issuers' endpoints answer
///    none.
///
/// The token is the caller's to have, good now: from this module's
/// own files, by [`Accounts::info`], or from a store of its own.
pub async fn info(http: &reqwest::Client, issuer: &Issuer, access_token: &str) -> Result<Value, Error> {
    let response = http.get(issuer.info_endpoint).bearer_auth(access_token).header(ACCEPT, "application/json").send().await.map_err(Error::Http)?;
    let status = response.status();
    // Rule 3.
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(Error::Refused { code: "unauthorized".to_string(), description: None });
    }
    let body = response.bytes().await.map_err(Error::Http)?;
    let unread = || Error::Answer { url: issuer.info_endpoint.to_string(), status: status.as_u16(), body: String::from_utf8_lossy(&body).into_owned() };
    if !status.is_success() {
        return Err(unread());
    }
    // Rule 1.
    serde_json::from_slice(&body).map_err(|_| unread())
}

impl Accounts {
    /// [`info`], of `account`: its access token had as every token is
    /// — see [`token`](Self::token): under the account's lock, renewed
    /// first where it has expired, and the lock let go before the
    /// issuer is asked. An issuer that refuses it is
    /// [`Error::SignedOut`].
    pub async fn info(&self, account: &Account) -> Result<Value, Error> {
        let issuer = issuer(&account.issuer)?;
        let token = self.token(account, Which::Access).await?;
        info(&self.http, issuer, &token).await.map_err(|error| match error {
            Error::Refused { .. } => Error::SignedOut(account.clone()),
            error => error,
        })
    }
}
