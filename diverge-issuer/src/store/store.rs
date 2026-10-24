//! Opening the database.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use super::Error;
use super::schema::SCHEMA;

/// The file's name under the root.
pub const FILE: &str = "issuer.sqlite";

/// The connections one process holds to the file.
const CONNECTIONS: u32 = 8;

/// `issuer.sqlite` at a root, open. Cloning it shares the
/// connections.
#[derive(Debug, Clone)]
pub struct Store {
    pub(crate) pool: SqlitePool,
}

impl Store {
    /// Open `<root>/issuer.sqlite`, creating it owner-only if it is
    /// absent, and make its tables where it has none.
    ///
    /// The issuer and the CLI open the one file at once — the CLI to
    /// add an identity or rotate the keys, the issuer to read them on every
    /// request — so the journal is WAL, and a write waits as long as
    /// another process's write holds the file: SQLite's longest wait,
    /// about 24 days, which is to say no deadline. Foreign keys are
    /// on, so removing an identity removes everything
    /// issued to it.
    pub async fn open(root: &Path) -> Result<Store, Error> {
        let path = root.join(FILE);
        create(&path).await?;
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_millis(i32::MAX.unsigned_abs().into()));
        let open = |source| Error::Open { path: path.clone(), source };
        let pool = SqlitePoolOptions::new().max_connections(CONNECTIONS).connect_with(options).await.map_err(open)?;
        sqlx::raw_sql(SCHEMA).execute(&pool).await.map_err(open)?;
        Ok(Store { pool })
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
