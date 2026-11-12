//! An account's file.

use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt as _;

use super::{Account, Accounts, Error, Tokens};

/// Everything an account's file holds: who it is, and what its issuer
/// answered its login, and every renewal since, with. No type outside
/// this module has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Record {
    /// The issuer's name.
    pub(super) issuer: String,
    /// The user's id at the issuer.
    pub(super) id: String,
    /// The tokens, their members beside the two above in the file.
    #[serde(flatten)]
    pub(super) tokens: Tokens,
}

/// The files of one account.
pub(super) struct Paths {
    /// The account: `<issuer>.<id>.json`.
    pub(super) record: PathBuf,
    /// Its lock: `<issuer>.<id>.lock`.
    pub(super) lock: PathBuf,
    /// Where a write is staged before it is renamed over the record:
    /// `.<issuer>.<id>.json.writing`.
    pub(super) staging: PathBuf,
}

impl Accounts {
    /// The files of `account`, under the accounts' folder.
    pub(super) fn paths(&self, account: &Account) -> Paths {
        Paths {
            record: self.folder.join(format!("{account}.json")),
            lock: self.folder.join(format!("{account}.lock")),
            staging: self.folder.join(format!(".{account}.json.writing")),
        }
    }
}

/// The record at `paths`, or `None` where there is no such file. The
/// caller holds the lock.
pub(super) async fn read(paths: &Paths) -> Result<Option<Record>, Error> {
    let bytes = match tokio::fs::read(&paths.record).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(Error::Io { path: paths.record.clone(), source }),
    };
    serde_json::from_slice(&bytes).map(Some).map_err(Error::Json)
}

/// Write `record` at `paths`: whole, to the staging file — made
/// owner-only — and renamed over the record, so a reader finds the
/// old file or the new one and never a part of either. The caller
/// holds the lock.
pub(super) async fn write(paths: &Paths, record: &Record) -> Result<(), Error> {
    let staging = &paths.staging;
    let failed = |source| Error::Io { path: staging.clone(), source };
    let bytes = serde_json::to_vec_pretty(record).map_err(Error::Json)?;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut staged = options.open(staging).await.map_err(failed)?;
    staged.write_all(&bytes).await.map_err(failed)?;
    staged.sync_all().await.map_err(failed)?;
    drop(staged);
    tokio::fs::rename(staging, &paths.record).await.map_err(|source| Error::Io { path: paths.record.clone(), source })
}
