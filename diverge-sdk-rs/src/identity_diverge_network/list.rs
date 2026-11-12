//! Every account.

use std::io;

use super::account::usable;
use super::{Account, Accounts, Error};

impl Accounts {
    /// Every account, by provider and then id, as the files are named:
    /// a `<provider>.<id>.json` whose id is one an account's may be.
    /// Any other file is passed over, the staging files among them.
    /// No file is read, and so no token.
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
            let Some((provider, id)) = stem.split_once('.') else {
                continue;
            };
            if !provider.is_empty() && usable(id) {
                accounts.push(Account { provider: provider.to_string(), id: id.to_string() });
            }
        }
        accounts.sort();
        Ok(accounts)
    }
}
