//! One question, the files out of a container or a volume, written
//! under a local path.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use diverge_sdk::daemon::client::stream::{ExecuteStream, OpenError};
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;

use super::exchange;
use crate::run::{Emitter, Error};

/// A download's request: what its endpoint's executor yields, one
/// implementation per endpoint.
pub trait Download {
    /// The stream's item: a chunk, or one of the answers.
    type Item: Downloaded + Unpin;
    /// Why an item could not be read.
    type Failure: std::error::Error + Send + Sync + Unpin + 'static;
    /// The request put: the endpoint's own `execute`.
    fn execute(
        &self,
        handle: &Handle,
    ) -> impl Future<
        Output = Result<ExecuteStream<Self::Item, Self::Failure>, OpenError<serde_json::Error>>,
    > + Send;
}

/// A download's item: a chunk, taken apart, or an answer printed as
/// it is.
pub trait Downloaded {
    /// The answer the item is when it is no chunk: the three every
    /// download has, or a volume's four.
    type Answer: Serialize;
    /// The chunk's path and body, or the answer the item is when it is
    /// no chunk.
    fn chunk(self) -> Result<(Vec<String>, Bytes), Self::Answer>;
}

/// What a download leaf prints, one line: an answer of the daemon's,
/// `A`, as it comes, or — once the stream has finished — one file
/// written, its path and its size. Untagged, so the line is the one
/// or the other as it is: `"not_found"`, `{"error":…}`,
/// `{"path":[…],"bytes":N}`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
pub enum Line<A> {
    /// An answer that is no chunk: the endpoint's own, see
    /// [`Answer`](super::Answer) for the three every download has.
    Answer(A),
    /// One file written under the local path.
    Written {
        /// The file's path, as components, as the daemon named it.
        path: Vec<String>,
        /// How many bytes it holds.
        bytes: u64,
    },
}

/// The request put, every chunk appended to its file under `into` —
/// `into` itself for the one file of a file download, `into/<path>`
/// for each file of a directory's, every parent made, each file
/// truncated at its first chunk — every other answer emitted as it
/// comes, and, once the stream has finished, one line per file
/// written: its path and its size — each a [`Line`].
pub async fn download<Q: Download>(
    handle: &Handle,
    request: &Q,
    into: &Path,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let mut items = request.execute(handle).await.map_err(exchange::open)?;
    let mut written: Vec<(Vec<String>, u64)> = Vec::new();
    let mut files: HashMap<Vec<String>, tokio::fs::File> = HashMap::new();
    while let Some(item) = items.next().await {
        let item = item.map_err(exchange::stream)?;
        let (path, body) = match item.chunk() {
            Ok(chunk) => chunk,
            Err(answer) => {
                emitter.emit(&Line::Answer(answer)).await?;
                continue;
            }
        };
        let file = match files.get_mut(&path) {
            Some(file) => file,
            None => {
                let local = local(into, &path);
                if let Some(parent) = local.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|source| Error::Io {
                            path: parent.to_path_buf(),
                            source,
                        })?;
                }
                let file = tokio::fs::File::create(&local)
                    .await
                    .map_err(|source| Error::Io {
                        path: local,
                        source,
                    })?;
                written.push((path.clone(), 0));
                files.entry(path.clone()).or_insert(file)
            }
        };
        file.write_all(&body).await.map_err(|source| Error::Io {
            path: local(into, &path),
            source,
        })?;
        if let Some(entry) = written.iter_mut().find(|(written, _)| *written == path) {
            entry.1 += body.len() as u64;
        }
    }
    for (path, file) in files {
        file.sync_all().await.map_err(|source| Error::Io {
            path: local(into, &path),
            source,
        })?;
    }
    for (path, bytes) in written {
        emitter
            .emit(&Line::<<Q::Item as Downloaded>::Answer>::Written { path, bytes })
            .await?;
    }
    Ok(())
}

/// Where a chunk's file lands: `into` itself for the empty path,
/// else `into` joined with every component.
fn local(into: &Path, path: &[String]) -> PathBuf {
    let mut local = into.to_path_buf();
    for component in path {
        local.push(component);
    }
    local
}
