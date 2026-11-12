//! An account's token, good now.

use std::io;

use super::account::usable;
use super::issuer::issuer;
use super::login::now;
use super::record::{read, write};
use super::{Account, Accounts, Error, Which};
use crate::file_lock;

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
    /// 4. **A token still good is answered** as it is, and the lock
    ///    let go: see [`Tokens::good`](super::Tokens::good).
    /// 5. **Any other is renewed** — see
    ///    [`Tokens::renew`](super::Tokens::renew) — the file written,
    ///    and the lock let go.
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
        if let Some(token) = record.tokens.good(which, now()) {
            let token = token.to_string();
            drop(held);
            return Ok(token);
        }
        // Rule 5.
        record.tokens.renew(&self.http, issuer, now()).await.map_err(|error| match error {
            Error::Refused { .. } => Error::SignedOut(account.clone()),
            error => error,
        })?;
        write(&paths, &record).await?;
        drop(held);
        // Rule 6.
        record.tokens.good(which, now()).map(str::to_string).ok_or_else(|| Error::SignedOut(account.clone()))
    }
}
