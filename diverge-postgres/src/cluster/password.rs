//! The superuser's password, minted once.

use std::io;
use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt as _;

use super::Error;

/// The file in the directory that holds the password: what `initdb`
/// is pointed at as its password file.
pub const PASSWORD: &str = "password";

/// The superuser password: `<dir>/password`, read with its surrounding
/// whitespace trimmed, or minted — the thirty-two hex digits of a v4
/// uuid — and written there, readable by its owner alone on Unix. It
/// is what `initdb` gives the superuser, and what the ready line
/// carries, so it is never changed here: a cluster initialized with
/// one password is reached with that one.
pub async fn password(dir: &Path) -> Result<String, Error> {
    let path = dir.join(PASSWORD);
    match tokio::fs::read_to_string(&path).await {
        Ok(found) => {
            let found = found.trim().to_string();
            if found.is_empty() {
                return Err(Error::EmptyPassword(path));
            }
            Ok(found)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => mint(path).await,
        Err(source) => Err(Error::Io { path, source }),
    }
}

/// Mint a password and write it, creating the file for this program
/// alone to read.
async fn mint(path: PathBuf) -> Result<String, Error> {
    let minted = uuid::Uuid::new_v4().simple().to_string();
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&path).await.map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    file.write_all(minted.as_bytes()).await.map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    file.flush().await.map_err(|source| Error::Io { path, source })?;
    Ok(minted)
}
