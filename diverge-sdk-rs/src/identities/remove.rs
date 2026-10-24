//! Signing an identity out of the host.

use std::io;

use super::{Error, Identities, Name};
use crate::file_lock;

impl Identities {
    /// Remove `name`'s file, under its lock. The answer is whether
    /// there was one. The lock file stays, as every lock file does,
    /// and the issuer is not told: its refresh token simply goes
    /// unused.
    pub async fn remove(&self, name: &Name) -> Result<bool, Error> {
        let paths = self.paths(name);
        match tokio::fs::metadata(&paths.folder).await {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(source) => return Err(Error::Io { path: paths.folder, source }),
        }
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        let removed = match tokio::fs::remove_file(&paths.record).await {
            Ok(()) => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(source) => return Err(Error::Io { path: paths.record, source }),
        };
        drop(held);
        Ok(removed)
    }
}
