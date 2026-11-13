//! Every account.

use std::io;

use super::account::usable;
use super::{Account, Accounts, Error};

impl Accounts {
    /// Whether `account` is one of the host's: whether it has a file.
    /// The file is not read, and no lock is taken. An account whose id
    /// could not name a file has none.
    pub async fn holds(&self, account: &Account) -> Result<bool, Error> {
        if account.issuer.is_empty() || account.issuer.contains('.') || !usable(&account.id) {
            return Ok(false);
        }
        let record = self.paths(account).record;
        match tokio::fs::metadata(&record).await {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(source) => Err(Error::Io { path: record, source }),
        }
    }

    /// Every account, by issuer and then id, as the files are named:
    /// a `<issuer>.<id>.json` whose id is one an account's may be.
    /// Any other file is passed over, the locks and the staging files
    /// among them. No file is read, so no token is, and no lock is
    /// taken.
    pub async fn list(&self) -> Result<Vec<Account>, Error> {
        let failed = |source| Error::Io { path: self.folder.clone(), source };
        let mut reader = match tokio::fs::read_dir(&self.folder).await {
            Ok(reader) => reader,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(failed(source)),
        };
        let mut accounts = Vec::new();
        while let Some(entry) = reader.next_entry().await.map_err(failed)? {
            let name = entry.file_name();
            let Some(stem) = name.to_str().and_then(|name| name.strip_suffix(".json")).filter(|stem| !stem.starts_with('.')) else {
                continue;
            };
            let Some((issuer, id)) = stem.split_once('.') else {
                continue;
            };
            if !issuer.is_empty() && usable(id) {
                accounts.push(Account { issuer: issuer.to_string(), id: id.to_string() });
            }
        }
        accounts.sort();
        Ok(accounts)
    }
}
