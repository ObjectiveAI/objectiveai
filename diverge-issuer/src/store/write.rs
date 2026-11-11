//! A transaction that writes.

use sqlx::{Sqlite, Transaction};

use super::{Error, Store};

impl Store {
    /// A transaction that takes the file's write lock at its `begin`,
    /// not at its first write: a transaction that reads and then
    /// writes on what it read — a poll, a rotation — would otherwise
    /// fail at once, under WAL, when another process wrote between its
    /// read and its write. The wait for the lock is the open's busy
    /// timeout.
    pub(crate) async fn write(&self) -> Result<Transaction<'static, Sqlite>, Error> {
        Ok(self.pool.begin_with("begin immediate").await?)
    }
}
