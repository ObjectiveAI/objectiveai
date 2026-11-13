//! A login waiting for a person, for an account kept in a file.

use serde::Deserialize;

use super::login::Device;
use super::record::{Record, write};
use super::{Account, Accounts, Error};
use crate::file_lock;

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

/// A login begun for the [`Accounts`]: what to show the person, and
/// the [`Device`] the wait is on.
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
    pub(super) device: Device,
}

impl Pending {
    /// Wait for the person to decide — see [`Device::wait`] — and
    /// write the account an approval is, replacing one already there:
    /// under its lock — see the [rules](super), rule 6 — taken here,
    /// once the account is known, and let go once the file is written.
    /// The answer is the account. Any other ending writes nothing.
    pub async fn wait(self) -> Result<Account, Error> {
        let issuer = self.device.issuer();
        let (id, tokens) = self.device.wait(&self.accounts.http).await?;
        let account = Account { issuer: issuer.name.to_string(), id: id.clone() };
        let record = Record { issuer: account.issuer.clone(), id, tokens };
        let paths = self.accounts.paths(&account);
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        write(&paths, &record).await?;
        drop(held);
        Ok(account)
    }
}
