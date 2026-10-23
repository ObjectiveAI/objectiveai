//! Opening the keyring.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use super::Error;
use super::schema::SCHEMA;

/// The file's name under the root.
pub const FILE: &str = "keyring.db";

/// The connections one process holds to the file.
const CONNECTIONS: u32 = 4;

/// The keyring at a root, open. Cloning it shares the connections.
#[derive(Debug, Clone)]
pub struct Keyring {
    pub(super) pool: SqlitePool,
}

impl Keyring {
    /// Open `<root>/keyring.db`, creating it owner-only if it is
    /// absent, and make its table if it has none.
    ///
    /// A write waits as long as another process's write holds the
    /// file — SQLite's longest wait, about 24 days, which is to say
    /// no deadline: a write is short, and one that never ends is a
    /// stuck process, not a slow file.
    pub async fn open(root: &Path) -> Result<Keyring, Error> {
        let path = root.join(FILE);
        create(&path).await?;
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(i32::MAX.unsigned_abs().into()));
        let open = |source| Error::Open { path: path.clone(), source };
        let pool = SqlitePoolOptions::new().max_connections(CONNECTIONS).connect_with(options).await.map_err(open)?;
        sqlx::query(SCHEMA).execute(&pool).await.map_err(open)?;
        Ok(Keyring { pool })
    }
}

/// Create the file empty and owner-only if it is absent; leave it as
/// it is if it is there. SQLite makes its log files with the
/// database's own permissions.
async fn create(path: &Path) -> Result<(), Error> {
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    match options.open(path).await {
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(source) => Err(Error::Create { path: PathBuf::from(path), source }),
    }
}
