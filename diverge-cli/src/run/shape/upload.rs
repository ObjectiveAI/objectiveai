//! One question, the files into a container or a volume, read from a
//! local path.

use std::path::{Path, PathBuf};

use bytes::Bytes;
use diverge_sdk::shared::error::Error as Refusal;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::decode::Decode;
use futures_util::stream::{self, Once};
use serde_json::Value;

use super::request;
use crate::run::{Emitter, Error};

/// What an upload's content is read as: one piece per file, the file
/// read whole once the daemon asks for it.
pub type Content = Once<std::pin::Pin<Box<dyn Future<Output = Result<Bytes, Refusal>> + Send>>>;

/// An upload's request: its endpoint's executor, one implementation
/// per endpoint, answering the response as JSON.
pub trait Upload: for<'a> Decode<'a> {
    /// The request put with `source` answering the daemon's asks: the
    /// endpoint's own `execute`, its answer as JSON.
    fn execute<F>(&self, handle: &Handle, source: F) -> impl Future<Output = Result<Value, Error>> + Send
    where
        F: FnMut(Option<String>) -> Option<Content> + Send;
}

/// The request put, every file the daemon asks for read from under
/// `from` — `from` itself for the one file of a file upload,
/// `from/<path>` for each file of a directory's — and the answer
/// emitted. A file that cannot be read refuses its channel, which the
/// daemon answers as the upload's error.
pub async fn upload<Q: Upload>(handle: &Handle, bytes: &[u8], from: &Path, emitter: &mut Emitter) -> Result<(), Error> {
    let request: Q = request(bytes)?;
    let from = from.to_path_buf();
    let source = move |asked: Option<String>| -> Option<Content> {
        let local = match asked {
            None => from.clone(),
            Some(path) => path.split('/').fold(from.clone(), |mut local, component| {
                local.push(component);
                local
            }),
        };
        Some(stream::once(Box::pin(read(local)) as std::pin::Pin<Box<dyn Future<Output = Result<Bytes, Refusal>> + Send>>))
    };
    let answer = request.execute(handle, source).await?;
    emitter.emit_value(answer).await?;
    Ok(())
}

/// The file, whole, as the one piece of its content; a file that
/// cannot be read as the piece's refusal, in the read's own words.
async fn read(local: PathBuf) -> Result<Bytes, Refusal> {
    tokio::fs::read(&local).await.map(Bytes::from).map_err(|error| {
        Refusal(serde_json::json!({
            "kind": "read",
            "path": local.display().to_string(),
            "error": error.to_string(),
        }))
    })
}
