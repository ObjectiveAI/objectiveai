//! A file's bytes, piece by piece.

use std::path::Path;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use diverge_sdk::CHUNK_SIZE;
use futures_util::{Stream, stream};
use tokio::fs::File;
use tokio::io::AsyncReadExt as _;

/// A file's bytes as pieces of at most [`CHUNK_SIZE`] each, in order,
/// ending at the file's end — or short where a read fails, which the
/// reader's digest check sees.
pub type Pieces = Pin<Box<dyn Stream<Item = Bytes> + Send>>;

/// The file at `path` as pieces, or `None` for one that could not be
/// opened.
pub async fn pieces(path: &Path) -> Option<Pieces> {
    let file = File::open(path).await.ok()?;
    Some(Box::pin(stream::unfold(Some(file), |file| async move {
        let mut file = file?;
        let mut buffer = BytesMut::with_capacity(CHUNK_SIZE);
        match file.read_buf(&mut buffer).await {
            Ok(0) | Err(_) => None,
            Ok(_) => Some((buffer.freeze(), Some(file))),
        }
    })))
}
