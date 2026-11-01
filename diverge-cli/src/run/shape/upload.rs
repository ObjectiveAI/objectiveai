//! One question, the files into a container or a volume, read from a
//! local path.

use std::path::{Path, PathBuf};
use std::pin::Pin;

use bytes::Bytes;
use diverge_sdk::shared::error::Error as Refusal;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::Stream;
use futures_util::stream;
use tokio::io::AsyncReadExt as _;
use serde::Serialize;

use crate::run::{Emitter, Error};

/// What an upload's content is read as: the file, opened once the
/// daemon asks for it and read in pieces of at most
/// [`CHUNK_SIZE`](diverge_sdk::CHUNK_SIZE) as the wire takes them —
/// never held whole.
pub type Content = Pin<Box<dyn Stream<Item = Result<Bytes, Refusal>> + Send>>;

/// An upload's request: its endpoint's executor, one implementation
/// per endpoint, answering the endpoint's own response frame.
pub trait Upload {
    /// The answer: the endpoint's own response frame.
    type Answer: Serialize;
    /// The request put with `source` answering the daemon's asks: the
    /// endpoint's own `execute`, its answer.
    fn execute<F>(
        &self,
        handle: &Handle,
        source: F,
    ) -> impl Future<Output = Result<Self::Answer, Error>> + Send
    where
        F: FnMut(Option<String>) -> Option<Content> + Send;
}

/// The request put, every file the daemon asks for read from under
/// `from` — `from` itself for the one file of a file upload,
/// `from/<path>` for each file of a directory's — and the answer
/// emitted. A file that cannot be read refuses its channel, which the
/// daemon answers as the upload's error.
pub async fn upload<Q: Upload>(
    handle: &Handle,
    request: &Q,
    from: &Path,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let from = from.to_path_buf();
    let source = move |asked: Option<String>| -> Option<Content> {
        let local = match asked {
            None => from.clone(),
            Some(path) => path.split('/').fold(from.clone(), |mut local, component| {
                local.push(component);
                local
            }),
        };
        Some(read(local))
    };
    let answer = request.execute(handle, source).await?;
    emitter.emit(&answer).await?;
    Ok(())
}

/// Where a file's reading stands.
enum Reading {
    /// Not yet opened.
    Start(PathBuf),
    /// Open, and read up to here.
    Open(tokio::fs::File, PathBuf),
    /// Read to its end, or failed.
    Done,
}

/// The file, in pieces as they are pulled; a file that cannot be
/// opened or read as one piece's refusal, in the read's own words,
/// and the end.
fn read(local: PathBuf) -> Content {
    Box::pin(stream::unfold(Reading::Start(local), |reading| async move {
        let (mut file, local) = match reading {
            Reading::Done => return None,
            Reading::Open(file, local) => (file, local),
            Reading::Start(local) => match tokio::fs::File::open(&local).await {
                Ok(file) => (file, local),
                Err(error) => return Some((Err(refusal(&local, &error)), Reading::Done)),
            },
        };
        let mut buffer = vec![0u8; diverge_sdk::CHUNK_SIZE];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(read) => {
                buffer.truncate(read);
                Some((Ok(Bytes::from(buffer)), Reading::Open(file, local)))
            }
            Err(error) => Some((Err(refusal(&local, &error)), Reading::Done)),
        }
    }))
}

/// A read that failed, as the content's refusal.
fn refusal(local: &Path, error: &std::io::Error) -> Refusal {
    Refusal(serde_json::json!({
        "kind": "read",
        "path": local.display().to_string(),
        "error": error.to_string(),
    }))
}
