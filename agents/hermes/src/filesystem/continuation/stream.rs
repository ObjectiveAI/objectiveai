//! The outbound stream: each file, a piece at a time, behind its tag.

use std::io;
use std::path::Path;

use futures_util::Stream;
use tokio::fs::File;
use tokio::io::AsyncReadExt as _;

use super::db::fold;
use super::{HERMES_HOME, PIECE, ReadError, STATE_DB, STATE_DB_TAG, TAGS, path_for};

/// Harvest the state under [`HERMES_HOME`] as the chunks the run
/// closes with — [`Ingest`](super::Ingest)'s inverse, a piece at a
/// time. Called after the gateway process has exited: nothing else
/// may hold the database.
///
/// The database is folded before anything is read: `VACUUM` first,
/// which rewrites every page (and reads every page, so damage
/// surfaces here) THROUGH the write-ahead log, then a
/// `wal_checkpoint(TRUNCATE)`, which moves the log's frames into
/// the main file and empties the log; then the connection is
/// closed, which — being the last — deletes the log. A checkpoint
/// that could not complete means something still holds the database
/// ([`ReadError::Busy`]). `VACUUM` needs up to twice the file's size
/// free under the home while it runs. That much is done before this
/// returns, so a fold that fails is an `Err` here and never a stream
/// that dies mid-way.
///
/// Then the stream: the three files in tag order, `state.db`
/// required and the memory files skipped when absent, each read
/// [`PIECE`] bytes at a time and yielded behind its tag as soon as
/// that piece is read — one piece alive at once. An empty file
/// yields nothing: to Hermes, absent and empty memory are the same.
/// The consumer sends each item as one continuation frame and drops
/// it.
pub async fn stream()
-> Result<impl Stream<Item = Result<Vec<u8>, ReadError>>, ReadError> {
    let db = Path::new(HERMES_HOME).join(STATE_DB);
    fold(&db).await?;

    Ok(async_stream::try_stream! {
        for tag in TAGS {
            let path = path_for(tag).expect("one of the three");
            let mut file = match File::open(&path).await {
                Ok(file) => file,
                Err(error)
                    if tag != STATE_DB_TAG
                        && error.kind() == io::ErrorKind::NotFound =>
                {
                    continue;
                }
                Err(error) => Err(ReadError::Io(error))?,
            };
            loop {
                // The tag, then up to PIECE bytes of the file: read
                // until the piece is full or the file is done.
                let mut piece = Vec::with_capacity(PIECE + 1);
                piece.push(tag);
                while piece.len() < PIECE + 1 {
                    if file.read_buf(&mut piece).await? == 0 {
                        break;
                    }
                }
                let done = piece.len() < PIECE + 1;
                if piece.len() > 1 {
                    yield piece;
                }
                if done {
                    break;
                }
            }
        }
    })
}
