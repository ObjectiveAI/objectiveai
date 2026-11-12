//! Accounts: listing a template's, and one's token, good now.

use std::time::{SystemTime, UNIX_EPOCH};

use diverge_sdk::identity_diverge_network::{Error, Issuer, Tokens};
use serde::Serialize;
use sqlx::{PgPool, Row as _};

use crate::failure::Failure;

/// One account, as the tool lists it: no token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    /// The account, written `<issuer>.<id>`: the first two parts of
    /// every subject it owns.
    pub account: String,
    /// Its issuer's name.
    pub issuer: String,
    /// Its id at that issuer.
    pub id: String,
}

/// Now, in seconds since the Unix epoch.
fn now() -> i64 {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or(0);
    i64::try_from(seconds).unwrap_or(i64::MAX)
}

/// Every account kept under `template`, by issuer and then id. No
/// token is read.
pub async fn list(pool: &PgPool, template: &str) -> Result<Vec<Account>, Failure> {
    let rows = sqlx::query("select issuer, id from identity_accounts where template = $1 order by issuer, id").bind(template).fetch_all(pool).await?;
    let mut accounts = Vec::with_capacity(rows.len());
    for row in rows {
        let (issuer, id): (String, String) = (row.try_get("issuer")?, row.try_get("id")?);
        accounts.push(Account { account: format!("{issuer}.{id}"), issuer, id });
    }
    Ok(accounts)
}

/// The token `identity.diverge.network` takes of the account `id` at
/// `issuer`, kept under `template`, good to use now. One transaction,
/// the row locked in it, which is this store's turn — what a file's
/// lock is to an account kept in one. In order:
///
/// 1. **The row is read, and locked**: `for update`, so that of two
///    calls that find a token expired, one renews it and the other
///    waits, and reads what the first wrote. No such row is
///    [`Failure::NoAccount`].
/// 2. **A token still good is answered** as it is — see
///    [`Tokens::good`] — and the transaction ended.
/// 3. **Any other is renewed** — see [`Tokens::renew`] — the row
///    written, and the transaction committed, before the token is
///    answered: the lock is let go before the request the token is
///    for.
/// 4. **Signed out.** A renewal the issuer refuses, and one that
///    answers a token no better, are [`Failure::SignedOut`], and the
///    row is left as it was.
pub async fn token(pool: &PgPool, http: &reqwest::Client, template: &str, issuer: &Issuer, id: &str) -> Result<String, Failure> {
    let account = format!("{}.{id}", issuer.name);
    let mut transaction = pool.begin().await?;
    // Rule 1.
    let row = sqlx::query("select access_token, access_expires, refresh_token, id_token from identity_accounts where template = $1 and issuer = $2 and id = $3 for update")
        .bind(template)
        .bind(issuer.name)
        .bind(id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| Failure::NoAccount(account.clone()))?;
    let mut tokens = Tokens {
        access_token: row.try_get("access_token")?,
        access_expires: row.try_get("access_expires")?,
        refresh_token: row.try_get("refresh_token")?,
        id_token: row.try_get("id_token")?,
    };
    // Rule 2.
    if let Some(token) = tokens.good(issuer.presents, now()) {
        return Ok(token.to_string());
    }
    // Rules 3 and 4.
    tokens.renew(http, issuer, now()).await.map_err(|error| match error {
        Error::Refused { .. } => Failure::SignedOut(account.clone()),
        error => Failure::Identity(error.to_string()),
    })?;
    let token = tokens.good(issuer.presents, now()).map(str::to_string).ok_or_else(|| Failure::SignedOut(account.clone()))?;
    sqlx::query("update identity_accounts set access_token = $4, access_expires = $5, refresh_token = $6, id_token = $7 where template = $1 and issuer = $2 and id = $3")
        .bind(template)
        .bind(issuer.name)
        .bind(id)
        .bind(&tokens.access_token)
        .bind(tokens.access_expires)
        .bind(&tokens.refresh_token)
        .bind(&tokens.id_token)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(token)
}
