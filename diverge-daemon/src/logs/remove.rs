//! Removing a log.

use std::path::Path;

use super::{Book, Error};

/// Remove `book`'s log directory, whole; one that was never made is
/// nothing to remove.
pub async fn remove<B: Book>(root: &Path, book: B) -> Result<(), Error> {
    let dir = book.dir(root);
    match tokio::fs::remove_dir_all(&dir).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::io(&dir, source)),
    }
}
