//! An account's file.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt as _;

use super::{Account, Accounts, Error};

/// Everything an account's file holds: who it is, and what the
/// provider answered its login with. No type outside this module has
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Record {
    /// The provider's name.
    pub(super) provider: String,
    /// The user's id at the provider.
    pub(super) id: String,
    /// The access token.
    pub(super) access_token: Option<String>,
    /// When the access token expires, in seconds since the Unix epoch,
    /// where the provider said; one that does not expire has none.
    pub(super) access_expires: Option<i64>,
    /// The refresh token, where the provider answered one.
    pub(super) refresh_token: Option<String>,
    /// The ID token, where the provider answered one.
    pub(super) id_token: Option<String>,
}

impl Accounts {
    /// Where `account`'s file is, and where a write of it is staged.
    fn paths(&self, account: &Account) -> (PathBuf, PathBuf) {
        (self.folder.join(format!("{account}.json")), self.folder.join(format!(".{account}.json.writing")))
    }

    /// Write `record` as its account's file: whole, to the staging
    /// file — made owner-only — and renamed over the file, so a reader
    /// finds the old file or the new one and never a part of either.
    pub(super) async fn write(&self, record: &Record) -> Result<(), Error> {
        let account = Account { provider: record.provider.clone(), id: record.id.clone() };
        let (file, staging) = self.paths(&account);
        let failed = |source| Error::Io { path: staging.clone(), source };
        let bytes = serde_json::to_vec_pretty(record).map_err(Error::Json)?;
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut staged = options.open(&staging).await.map_err(failed)?;
        staged.write_all(&bytes).await.map_err(failed)?;
        staged.sync_all().await.map_err(failed)?;
        drop(staged);
        tokio::fs::rename(&staging, &file).await.map_err(|source| Error::Io { path: file.clone(), source })
    }
}
