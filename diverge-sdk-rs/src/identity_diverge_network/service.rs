//! `identity.diverge.network`, asked as an account.

use reqwest::Method;
use reqwest::header::ACCEPT;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::issuer::issuer;
use super::{Account, Accounts, Error, Issuer};

/// The header that names the issuer the account's token is from.
pub const ISSUER_HEADER: &str = "diverge-issuer";

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

/// `POST /device/{user_code}/approve`'s answer.
#[derive(Deserialize)]
struct Approved {
    subject: String,
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

/// Approve the device's login whose user code is `user_code`, as the
/// subject named `name` of the account `token` is — a token of
/// `issuer`'s, the one `identity.diverge.network` takes, see
/// [`Issuer::presents`](super::Issuer) — at `service`, where
/// `identity.diverge.network` is: `POST /device/{user_code}/approve`.
/// The answer is the subject the device is signed in as, whole. A
/// code that is no waiting login's is [`Error::Service`], `404
/// not_found`.
///
/// The token is the caller's to have: from this module's own files,
/// by [`Accounts::approve`], or from a store of its own.
///
/// Approving a code signs its device in as the subject, whoever's
/// device it is: this is for a code the caller began itself, a moment
/// before, and for no other.
pub async fn approve(http: &reqwest::Client, service: &str, issuer: &Issuer, token: &str, user_code: &str, name: &str) -> Result<String, Error> {
    let (url, status, body) = request(http, service, issuer, token, Method::POST, &["device", user_code, "approve"], Some(&json!({ "name": name }))).await?;
    let approved: Approved = serde_json::from_slice(&body).map_err(|_| Error::Answer { url, status, body: String::from_utf8_lossy(&body).into_owned() })?;
    Ok(approved.subject)
}

/// One request of `identity.diverge.network`, at `service`, as the
/// account `token` is, with `json` as its body where there is one:
/// the URL asked, the status, and the body of a success. The rules:
///
/// 1. **The path** is `segments`, each one part: what a caller gives
///    — a user code, a name, an id — is escaped as a part, so none of
///    it can name another route.
/// 2. **As the account.** `Authorization: Bearer` carries `token`,
///    and [`ISSUER_HEADER`] names the issuer it is from.
/// 3. **A refusal** is any status that is no success, read as
///    `identity.diverge.network` writes every one,
///    `{"error","error_description"}`: [`Error::Service`]. A body that
///    is not that is [`Error::Answer`].
/// 4. **Nowhere else.** A client that follows no redirect sends the
///    token to the address asked and to no other; this module's own
///    is such a client, and a caller's own should be.
async fn request(http: &reqwest::Client, service: &str, issuer: &Issuer, token: &str, method: Method, segments: &[&str], json: Option<&Value>) -> Result<(String, u16, Vec<u8>), Error> {
    // Rule 1.
    let mut url = reqwest::Url::parse(service).map_err(|_| Error::Address(service.to_string()))?;
    url.path_segments_mut().map_err(|()| Error::Address(service.to_string()))?.pop_if_empty().extend(segments);
    // Rule 2.
    let mut request = http.request(method, url.clone()).bearer_auth(token).header(ISSUER_HEADER, issuer.name).header(ACCEPT, "application/json");
    if let Some(json) = json {
        request = request.json(json);
    }
    let response = request.send().await.map_err(Error::Http)?;
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

impl Accounts {
    /// [`approve`], as `account`: its token had from its file.
    pub async fn approve(&self, account: &Account, user_code: &str, name: &str) -> Result<String, Error> {
        let issuer = issuer(&account.issuer)?;
        let token = self.token(account, issuer.presents).await?;
        approve(&self.http, &self.service, issuer, &token, user_code, name).await
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
        self.send(account, Method::DELETE, &["subjects", name, "refresh-tokens", id], None).await?;
        Ok(())
    }

    /// [`send`](Self::send), and its answer read as `T`.
    async fn ask<T: DeserializeOwned>(&self, account: &Account, method: Method, segments: &[&str]) -> Result<T, Error> {
        let (url, status, body) = self.send(account, method, segments, None).await?;
        serde_json::from_slice(&body).map_err(|_| Error::Answer { url, status, body: String::from_utf8_lossy(&body).into_owned() })
    }

    /// One [`request`] of `identity.diverge.network`, as `account`:
    /// the token its issuer has `identity.diverge.network` take — see
    /// [`Issuer::presents`](super::Issuer) — had as every token is,
    /// see [`token`](Self::token): under the account's lock, renewed
    /// first where it has expired, the lock let go before the request
    /// is made.
    async fn send(&self, account: &Account, method: Method, segments: &[&str], json: Option<&Value>) -> Result<(String, u16, Vec<u8>), Error> {
        let issuer = issuer(&account.issuer)?;
        let token = self.token(account, issuer.presents).await?;
        request(&self.http, &self.service, issuer, &token, method, segments, json).await
    }
}
