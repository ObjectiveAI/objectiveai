//! An account's token, good now.

use std::io;

use serde_json::Value;

use super::account::usable;
use super::device::{Answer, post};
use super::issuer::issuer;
use super::login::now;
use super::pending::Tokens;
use super::record::{Record, read, write};
use super::who::claims;
use super::{Account, Accounts, Error, Issuer, Which};
use crate::file_lock;

/// The seconds before its expiry a token is renewed at: one that
/// would end while the request it is for is on its way is not used.
const EARLY: i64 = 60;

impl Accounts {
    /// `account`'s token, `which` of the two, good to use now: the one
    /// way a token is had, by every request made as an account — see
    /// the [rules](super), rules 6 and 7. In order:
    ///
    /// 1. **The account is there.** One whose issuer is none is
    ///    [`Error::Issuer`]; one whose id is none, or which has no
    ///    file, is [`Error::Missing`] — found before any lock is
    ///    taken, so that asking after an account that is not there
    ///    leaves no lock file behind.
    /// 2. **The lock is taken**, waited for while another holds it.
    /// 3. **The file is read**, under the lock.
    /// 4. **A token good for longer than [`EARLY`] is answered** as it
    ///    is, and the lock let go: see [`good`].
    /// 5. **Any other is renewed**: the refresh token spent at the
    ///    issuer's token endpoint, RFC 6749 §6, as the client
    ///    `identity.diverge.network` is there. What the issuer
    ///    answered — the access token and its expiry, and the ID token
    ///    and the refresh token where it answered new ones — is
    ///    written to the file, and the lock let go.
    /// 6. **Signed out.** An account with no such token, one whose
    ///    token expired with no refresh token to renew it by, a
    ///    renewal the issuer refuses, and one that answers a token no
    ///    better, are [`Error::SignedOut`]. The file is kept.
    pub(super) async fn token(&self, account: &Account, which: Which) -> Result<String, Error> {
        // Rule 1.
        let issuer = issuer(&account.issuer)?;
        if !usable(&account.id) {
            return Err(Error::Missing(account.clone()));
        }
        let paths = self.paths(account);
        match tokio::fs::metadata(&paths.record).await {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(Error::Missing(account.clone())),
            Err(source) => return Err(Error::Io { path: paths.record, source }),
        }
        // Rule 2.
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        // Rule 3.
        let mut record = read(&paths).await?.ok_or_else(|| Error::Missing(account.clone()))?;
        // Rule 4.
        if let Some(token) = good(&record, which) {
            drop(held);
            return Ok(token);
        }
        // Rule 5.
        renew(&self.http, issuer, &mut record).await.map_err(|error| match error {
            Error::Refused { .. } => Error::SignedOut(account.clone()),
            error => error,
        })?;
        write(&paths, &record).await?;
        drop(held);
        // Rule 6.
        good(&record, which).ok_or_else(|| Error::SignedOut(account.clone()))
    }
}

/// `record`'s token, `which` of the two, where it has one good for
/// longer than [`EARLY`] from now. The rules:
///
/// 1. **The access token** expires when the issuer said it would; one
///    the issuer gave no expiry — GitHub's — does not expire.
/// 2. **The ID token** expires at its own `exp`, read from it; one
///    with none is not good.
fn good(record: &Record, which: Which) -> Option<String> {
    let bound = now().saturating_add(EARLY);
    match which {
        Which::Access => record.access_token.clone().filter(|_| record.access_expires.is_none_or(|expires| expires > bound)),
        Which::Id => {
            let token = record.id_token.clone()?;
            let expires = claims(&token)?.get("exp").and_then(Value::as_i64)?;
            (expires > bound).then_some(token)
        }
    }
}

/// Renew `record`'s tokens at `issuer`: its refresh token spent, and
/// what was answered taken in. A record with no refresh token, and an
/// answer with no access token, are a refusal — [`Error::Refused`] —
/// as the issuer's own is. The caller holds the lock, and writes the
/// record.
async fn renew(http: &reqwest::Client, issuer: &Issuer, record: &mut Record) -> Result<(), Error> {
    let refused = |code: &str| Error::Refused { code: code.to_string(), description: None };
    let refresh_token = record.refresh_token.clone().ok_or_else(|| refused("no_refresh_token"))?;
    let mut form = vec![("grant_type", "refresh_token"), ("refresh_token", refresh_token.as_str()), ("client_id", issuer.client_id)];
    if let Some(secret) = issuer.client_secret {
        form.push(("client_secret", secret));
    }
    let tokens = match post::<Tokens>(http, issuer.token_endpoint, &form).await? {
        Answer::Answered(tokens) => tokens,
        Answer::Refused { code, description } => return Err(Error::Refused { code, description }),
    };
    record.access_token = Some(tokens.access_token.ok_or_else(|| refused("no_access_token"))?);
    record.access_expires = tokens.expires_in.map(|seconds| now().saturating_add(i64::try_from(seconds).unwrap_or(i64::MAX)));
    if let Some(id_token) = tokens.id_token {
        record.id_token = Some(id_token);
    }
    if let Some(refresh_token) = tokens.refresh_token {
        record.refresh_token = Some(refresh_token);
    }
    Ok(())
}
