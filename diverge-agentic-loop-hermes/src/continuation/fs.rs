//! Filesystem odds and ends the sink and the fold share.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// A SQLite sidecar's path: the database's own name with the suffix
/// appended, `state.db-wal` for `state.db`.
pub(super) fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = db.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Remove a file that may not exist; absence is fine.
pub(super) async fn remove_if_present(path: &Path) -> io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
