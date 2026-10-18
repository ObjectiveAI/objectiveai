//! Reading an agent's log: its length, and a span of it.

use std::io::SeekFrom;
use std::path::Path;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::ItemWrapper;
use tokio::fs::File;
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};

use super::{ENTRY, Error, dir};
use crate::store::AgentId;

/// How many items the agent's log holds: the `logs_index` of its
/// latest, `0` for a log never appended to.
pub async fn count(root: &Path, id: AgentId) -> Result<u64, Error> {
    let index_path = dir(root, id).join("index");
    match tokio::fs::metadata(&index_path).await {
        Ok(meta) => Ok(meta.len() / ENTRY),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(source) => Err(Error::io(&index_path, source)),
    }
}

/// The items numbered `from` through `to`, inclusive, in order —
/// clipped to the log, so a span past its end is what there is, and
/// an empty span is nothing.
pub async fn read(root: &Path, id: AgentId, from: u64, to: u64) -> Result<Vec<ItemWrapper>, Error> {
    let count = count(root, id).await?;
    let from = from.max(1);
    let to = to.min(count);
    if from > to {
        return Ok(Vec::new());
    }
    let dir = dir(root, id);
    let index_path = dir.join("index");
    let log_path = dir.join("log");
    let mut index = File::open(&index_path).await.map_err(|source| Error::io(&index_path, source))?;
    index
        .seek(SeekFrom::Start((from - 1) * ENTRY))
        .await
        .map_err(|source| Error::io(&index_path, source))?;
    let entries = usize::try_from(to - from + 1).unwrap_or(usize::MAX);
    let mut raw = vec![0u8; entries * 16];
    index.read_exact(&mut raw).await.map_err(|source| Error::io(&index_path, source))?;
    let mut log = File::open(&log_path).await.map_err(|source| Error::io(&log_path, source))?;
    let mut items = Vec::with_capacity(entries);
    for entry in raw.chunks_exact(16) {
        let offset = u64::from_be_bytes(entry[..8].try_into().unwrap_or([0; 8]));
        let length = u64::from_be_bytes(entry[8..].try_into().unwrap_or([0; 8]));
        log.seek(SeekFrom::Start(offset)).await.map_err(|source| Error::io(&log_path, source))?;
        let mut line = vec![0u8; usize::try_from(length).unwrap_or(0)];
        log.read_exact(&mut line).await.map_err(|source| Error::io(&log_path, source))?;
        items.push(serde_json::from_slice(&line).map_err(Error::Json)?);
    }
    Ok(items)
}
