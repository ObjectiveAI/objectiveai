//! The inbound sink: chunks appended to their files as they land.

use std::io;
use std::path::Path;

use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt as _;

use super::{HERMES_HOME, IngestError, MEMORIES, STATE_DB_TAG, path_for};

/// A continuation being written to disk as it arrives.
///
/// Started on the first chunk, fed one chunk at a time, finished
/// once. Each chunk's payload is appended to the file its tag names
/// the moment it lands; between chunks nothing is held but the open
/// handle of the file currently being written. Two refusals: a chunk
/// with no bytes at all has no tag ([`IngestError::Empty`]), and a
/// tag that is none of the three
/// ([`UnknownTag`](IngestError::UnknownTag)); and one judged at the
/// finish — a continuation with no `state.db` is no continuation
/// ([`MissingStateDb`](IngestError::MissingStateDb)).
pub struct Ingest {
    /// The file the current tag appends to, and which tag it is —
    /// kept open across same-tag chunks, reopened on a change.
    open: Option<(u8, File)>,
    /// Whether any `state.db` chunk has landed.
    state_db: bool,
}

impl Ingest {
    /// Make room: the memory directory, which Hermes has not created
    /// yet. The container is fresh — nothing stale exists to clear.
    pub async fn start() -> io::Result<Self> {
        tokio::fs::create_dir_all(Path::new(HERMES_HOME).join(MEMORIES))
            .await?;
        Ok(Ingest {
            open: None,
            state_db: false,
        })
    }

    /// Land one chunk: append its payload to the file its tag names.
    pub async fn push(&mut self, chunk: &[u8]) -> Result<(), IngestError> {
        let (tag, payload) = chunk.split_first().ok_or(IngestError::Empty)?;
        let path = path_for(*tag).ok_or(IngestError::UnknownTag(*tag))?;
        if self.open.as_ref().is_none_or(|(open, _)| *open != *tag) {
            self.close_open().await?;
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await?;
            self.open = Some((*tag, file));
            self.state_db |= *tag == STATE_DB_TAG;
        }
        let (_, file) = self.open.as_mut().expect("opened just above");
        file.write_all(payload).await?;
        Ok(())
    }

    /// The delivery is complete: flush and close the last file. A
    /// delivery that never carried `state.db` fails here — there is
    /// nothing to resume from.
    pub async fn finish(mut self) -> Result<(), IngestError> {
        self.close_open().await?;
        if !self.state_db {
            return Err(IngestError::MissingStateDb);
        }
        Ok(())
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
