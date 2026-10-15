//! A download: every file at a path, as chunks.

use std::future::Future;
use std::pin::Pin;

use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;

use crate::daemon::Daemon;
use crate::transfers::{Fail, Source};
use crate::volumes::{self, Found};

/// How a family sends one chunk: its path relative to what was asked
/// for, and a piece of the file.
pub type SendChunk<'a> = &'a (dyn for<'b> Fn(&'b ScopeHandle, &'b [String], &'b [u8]) -> Pin<Box<dyn Future<Output = ()> + Send + 'b>> + Sync);

/// Send what is at `path` in the source: a file as chunks with the
/// empty path; a directory as every file under it in bytewise order
/// of their paths, each file's chunks carrying its path; a zero-byte
/// file one empty chunk; a directory with no file nothing. Nothing at
/// the path is `NotFound` before anything is sent; a read that fails
/// midway is the error after what was sent.
pub async fn download(scope: &ScopeHandle, daemon: &Daemon, source: &Source, path: &[String], send: SendChunk<'_>) -> Result<(), Fail> {
    let files: Vec<Vec<String>> = match source.entry_at(daemon, path).await? {
        Found::File(_) => vec![Vec::new()],
        Found::Directory(nodes) => volumes::files_under(&nodes),
        Found::Missing => return Err(Fail::NotFound),
    };
    for relative in files {
        let mut full = path.to_vec();
        full.extend_from_slice(&relative);
        let mut pieces = source.read(daemon, &full).await?;
        let mut sent = false;
        while let Some(piece) = pieces.next().await {
            let piece = piece.map_err(Fail::Error)?;
            send(scope, &relative, &piece).await;
            sent = true;
        }
        if !sent {
            send(scope, &relative, &[]).await;
        }
    }
    Ok(())
}
