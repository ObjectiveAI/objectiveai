//! Appending one item to a log.

use std::path::Path;

use chrono::Utc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt as _;

use super::{Book, ENTRY, Entry, Error};

/// Append `item` to `book`'s log under `root`, numbered one past the
/// last and stamped now, and hand it back as the wire sends it. Makes
/// the directory on the first append. The caller holds the log's live
/// lock across this, so that two appends land one after the other and
/// the index entry counts are right.
pub async fn append<B: Book>(root: &Path, book: B, item: <B::Entry as Entry>::Item) -> Result<B::Entry, Error> {
    let dir = book.dir(root);
    tokio::fs::create_dir_all(&dir).await.map_err(|source| Error::io(&dir, source))?;
    let log_path = dir.join("log");
    let index_path = dir.join("index");
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await
        .map_err(|source| Error::io(&log_path, source))?;
    let mut index = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&index_path)
        .await
        .map_err(|source| Error::io(&index_path, source))?;
    let offset = log.metadata().await.map_err(|source| Error::io(&log_path, source))?.len();
    let entries = index.metadata().await.map_err(|source| Error::io(&index_path, source))?.len() / ENTRY;
    let wrapper = B::Entry::wrap(entries + 1, Utc::now(), item);
    let mut line = serde_json::to_vec(&wrapper).map_err(Error::Json)?;
    line.push(b'\n');
    log.write_all(&line).await.map_err(|source| Error::io(&log_path, source))?;
    log.flush().await.map_err(|source| Error::io(&log_path, source))?;
    let mut entry = [0u8; 16];
    entry[..8].copy_from_slice(&offset.to_be_bytes());
    entry[8..].copy_from_slice(&(line.len() as u64).to_be_bytes());
    index.write_all(&entry).await.map_err(|source| Error::io(&index_path, source))?;
    index.flush().await.map_err(|source| Error::io(&index_path, source))?;
    Ok(wrapper)
}
