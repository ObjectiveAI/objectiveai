//! `identity.diverge.network`, asked as an account.

use reqwest::Method;
use reqwest::header::ACCEPT;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::issuer::issuer;
use super::{Account, Accounts, Error};

/// The header that names the issuer the account's token is from.
pub const ISSUER_HEADER: &str = "diverge-issuer";

/// A device's login at `identity.diverge.network` that waits to be
/// decided: what its owner is shown of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Login {
    /// The login's user code.
    pub user_code: String,
    /// The address the device asked from.
    pub ip: String,
    /// The `User-Agent` the device asked with, where it sent one.
    pub user_agent: Option<String>,
    /// The scope the login grants.
    pub scope: String,
    /// When the device asked, in seconds since the Unix epoch.
    pub created: i64,
    /// When the login expires.
    pub expires: i64,
}

/// One subject of an account's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    /// The subject, whole: `<issuer>.<id>.<name>`, the `sub` of its
    /// tokens.
    pub subject: String,
    /// Its name: the third part.
    pub name: String,
    /// How many refresh tokens it has that have not been ended.
    pub refresh_tokens: u64,
}

/// One of a subject's refresh tokens, described: where it was signed
/// in and where it was last spent, and never the token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshToken {
    /// The refresh token's id: no token, and what it is ended by.
    pub id: String,
    /// When the login that made it was approved, in seconds since the
    /// Unix epoch.
    pub created: i64,
    /// The address the device asked for its code from.
    pub ip: String,
    /// The `User-Agent` the device asked with, where it sent one.
    pub user_agent: Option<String>,
    /// When it was last spent; absent while it never was.
    pub last_used: Option<i64>,
    /// The address it was last spent from.
    pub last_ip: Option<String>,
}

/// `GET /subjects`' answer.
#[derive(Deserialize)]
struct Subjects {
    subjects: Vec<Subject>,
}

/// `GET /subjects/{name}/refresh-tokens`' answer.
#[derive(Deserialize)]
struct RefreshTokens {
    refresh_tokens: Vec<RefreshToken>,
}

impl Accounts {
    /// The login whose user code is `user_code`, as it waits: `GET
    /// /device/{user_code}`. One that does not wait — no login's,
    /// decided, expired — is [`Error::Service`], `404 not_found`.
    pub async fn waiting(&self, account: &Account, user_code: &str) -> Result<Login, Error> {
        self.ask(account, Method::GET, &["device", user_code]).await
    }

    /// Every subject of `account`'s that is signed in somewhere, by
    /// name: `GET /subjects`.
    pub async fn subjects(&self, account: &Account) -> Result<Vec<Subject>, Error> {
        let answer: Subjects = self.ask(account, Method::GET, &["subjects"]).await?;
        Ok(answer.subjects)
    }

    /// Every refresh token of `account`'s subject named `name`, the
    /// oldest first: `GET /subjects/{name}/refresh-tokens`.
    pub async fn refresh_tokens(&self, account: &Account, name: &str) -> Result<Vec<RefreshToken>, Error> {
        let answer: RefreshTokens = self.ask(account, Method::GET, &["subjects", name, "refresh-tokens"]).await?;
        Ok(answer.refresh_tokens)
    }

    /// End the refresh token `id` of `account`'s subject named
    /// `name`: `DELETE /subjects/{name}/refresh-tokens/{id}`. One that
    /// is none of that subject's is [`Error::Service`], `404
    /// not_found`.
    pub async fn end(&self, account: &Account, name: &str, id: &str) -> Result<(), Error> {
        self.send(account, Method::DELETE, &["subjects", name, "refresh-tokens", id]).await?;
        Ok(())
    }

    /// [`send`](Self::send), and its answer read as `T`.
    async fn ask<T: DeserializeOwned>(&self, account: &Account, method: Method, segments: &[&str]) -> Result<T, Error> {
        let (url, status, body) = self.send(account, method, segments).await?;
        serde_json::from_slice(&body).map_err(|_| Error::Answer { url, status, body: String::from_utf8_lossy(&body).into_owned() })
    }

    /// One request of `identity.diverge.network`, as `account`: the
    /// URL asked, the status, and the body of a success. The rules:
    ///
    /// 1. **The path** is `segments`, each one part: what a caller
    ///    gives — a user code, a name, an id — is escaped as a part,
    ///    so none of it can name another route.
    /// 2. **As the account.** `Authorization: Bearer` carries the
    ///    token the account's issuer has `identity.diverge.network`
    ///    take — see [`Issuer::presents`](super::Issuer) — had as
    ///    every token is, see [`token`](Self::token): under the
    ///    account's lock, renewed first where it has expired, the lock
    ///    let go before this request is made. [`ISSUER_HEADER`] names
    ///    the issuer.
    /// 3. **A refusal** is any status that is no success, read as
    ///    `identity.diverge.network` writes every one,
    ///    `{"error","error_description"}`: [`Error::Service`]. A body
    ///    that is not that is [`Error::Answer`].
    /// 4. **Nowhere else.** The client follows no redirect, so the
    ///    token goes to the address asked and to no other.
    async fn send(&self, account: &Account, method: Method, segments: &[&str]) -> Result<(String, u16, Vec<u8>), Error> {
        // Rule 1.
        let mut url = reqwest::Url::parse(&self.service).map_err(|_| Error::Address(self.service.clone()))?;
        url.path_segments_mut().map_err(|()| Error::Address(self.service.clone()))?.pop_if_empty().extend(segments);
        // Rule 2.
        let issuer = issuer(&account.issuer)?;
        let token = self.token(account, issuer.presents).await?;
        let response = self.http.request(method, url.clone()).bearer_auth(token).header(ISSUER_HEADER, issuer.name).header(ACCEPT, "application/json").send().await.map_err(Error::Http)?;
        let status = response.status();
        let body = response.bytes().await.map_err(Error::Http)?.to_vec();
        if status.is_success() {
            return Ok((url.to_string(), status.as_u16(), body));
        }
        // Rule 3.
        let refusal: Option<Value> = serde_json::from_slice(&body).ok();
        match refusal.as_ref().and_then(|refusal| refusal.get("error")).and_then(Value::as_str) {
            Some(code) => Err(Error::Service {
                status: status.as_u16(),
                code: code.to_string(),
                description: refusal.as_ref().and_then(|refusal| refusal.get("error_description")).and_then(Value::as_str).map(str::to_string),
            }),
            None => Err(Error::Answer { url: url.to_string(), status: status.as_u16(), body: String::from_utf8_lossy(&body).into_owned() }),
        }
    }
}
