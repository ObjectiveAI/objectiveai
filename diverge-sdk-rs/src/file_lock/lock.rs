//! Taking the lock, waiting for it or not.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use super::{Error, Held, platform};

/// Take the exclusive lock on the file at `path`, waiting as long as
/// another process holds it. The file is created if absent, and its
/// contents are left as they are. The wait is a blocking system call,
/// made on a blocking task so the runtime is not held.
pub async fn wait_exclusive(path: impl AsRef<Path>) -> Result<Held, Error> {
    let path = path.as_ref().to_path_buf();
    match take(path.clone(), true).await? {
        Some(held) => Ok(held),
        // A waiting lock call returns only with the lock or an error;
        // "would block" is not among its answers.
        None => Err(Error::Lock {
            path,
            source: std::io::Error::other("a waiting lock returned without the lock"),
        }),
    }
}

/// Take the exclusive lock on the file at `path` if nobody holds it,
/// and answer `None` at once if somebody does. The file is created if
/// absent, and its contents are left as they are.
pub async fn try_exclusive(path: impl AsRef<Path>) -> Result<Option<Held>, Error> {
    take(path.as_ref().to_path_buf(), false).await
}

/// Open the file and lock it, on a blocking task.
async fn take(path: PathBuf, wait: bool) -> Result<Option<Held>, Error> {
    let on_task = path.clone();
    tokio::task::spawn_blocking(move || {
        let path = on_task;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| Error::Open {
                path: path.clone(),
                source,
            })?;
        match platform::lock(&file, wait) {
            Ok(true) => Ok(Some(Held::new(path, file))),
            Ok(false) => Ok(None),
            Err(source) => Err(Error::Lock { path, source }),
        }
    })
    .await
    .map_err(|source| Error::Blocking { path, source })?
}
