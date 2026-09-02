//! The inbound sink: chunks appended to their files as they land.

use std::io;
use std::path::Path;

use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt as _;

use super::fs::{remove_if_present, sidecar};
use super::{HERMES_HOME, IngestError, Landed, MEMORIES, STATE_DB, path_for};

/// A continuation being written to disk as it arrives.
///
/// Started on the first chunk, fed one chunk at a time, finished
/// once. Each chunk's payload is appended to the file its tag names
/// the moment it lands; between chunks nothing is held but the open
/// handle of the file currently being written. The rules a delivery
/// must keep, each with its refusal: a chunk with no bytes at all
/// has no tag ([`IngestError::Empty`]); a tag that is none of the
/// three ([`UnknownTag`](IngestError::UnknownTag)); a file's chunks
/// must be contiguous and the files in ascending tag order, so a tag
/// lower than the last seen is a sequence reordered or interleaved
/// ([`Order`](IngestError::Order)); and a continuation with no
/// `state.db` is no continuation
/// ([`MissingStateDb`](IngestError::MissingStateDb), judged at the
/// finish).
pub struct Ingest {
    /// The tag before this one, for the order rule.
    last: Option<u8>,
    /// The file the current tag appends to, and which tag it is.
    open: Option<(u8, File)>,
    /// Which files exist so far.
    landed: Landed,
}

impl Ingest {
    /// Make room: the memory directory created, and any stale
    /// `state.db` family removed — a leftover write-ahead log beside
    /// a fresh copy would be replayed INTO it. Called by the store on
    /// the first chunk, so a fresh start (no chunks) touches nothing.
    pub async fn start() -> io::Result<Self> {
        let home = Path::new(HERMES_HOME);
        tokio::fs::create_dir_all(home.join(MEMORIES)).await?;
        let db = home.join(STATE_DB);
        remove_if_present(&db).await?;
        for suffix in ["-wal", "-shm", "-journal"] {
            remove_if_present(&sidecar(&db, suffix)).await?;
        }
        Ok(Ingest {
            last: None,
            open: None,
            landed: Landed::NONE,
        })
    }

    /// Land one chunk: judge its tag, append its payload to the
    /// tag's file. A new tag closes the file before it and opens
    /// its own — created even when this chunk is tag-only, so an
    /// empty file round-trips as present.
    pub async fn push(&mut self, chunk: &[u8]) -> Result<(), IngestError> {
        let (tag, payload) = chunk.split_first().ok_or(IngestError::Empty)?;
        let path = path_for(*tag).ok_or(IngestError::UnknownTag(*tag))?;
        if let Some(after) = self.last
            && *tag < after
        {
            return Err(IngestError::Order { tag: *tag, after });
        }
        self.last = Some(*tag);

        if self.open.as_ref().is_none_or(|(open, _)| *open != *tag) {
            self.close_open().await?;
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await?;
            self.open = Some((*tag, file));
            self.landed.mark(*tag);
        }
        let (_, file) = self.open.as_mut().expect("opened just above");
        file.write_all(payload).await?;
        Ok(())
    }

    /// The delivery is complete: flush and close the last file, and
    /// say what landed. A delivery that never carried `state.db`
    /// fails here — there is nothing to resume from.
    pub async fn finish(mut self) -> Result<Landed, IngestError> {
        self.close_open().await?;
        if !self.landed.state_db {
            return Err(IngestError::MissingStateDb);
        }
        Ok(self.landed)
    }

    /// Flush and sync the open file, if any, and let it go.
    async fn close_open(&mut self) -> io::Result<()> {
        if let Some((_, mut file)) = self.open.take() {
            file.flush().await?;
            file.sync_all().await?;
        }
        Ok(())
    }
}
