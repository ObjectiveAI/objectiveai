//! An upload, read off the channels the daemon opens.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::resources::Kind;
use diverge_sdk::daemon::endpoints::resources::upload::client::request;
use diverge_sdk::daemon::endpoints::resources::upload::server::channel_request;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Error, ingest, paths, pieces};

/// What an upload came to: the bytes in an incoming directory, hashed.
#[derive(Debug)]
pub struct Received {
    /// The incoming directory holding the content: the file itself for
    /// a file resource, the tree for a directory one.
    pub incoming: PathBuf,
    /// Which kind.
    pub kind: Kind,
    /// The id the bytes hash to.
    pub id: String,
    /// The file's length, or the sum of the directory's files'.
    pub bytes: u64,
}

/// Read the upload the request describes off `scope`, into a fresh
/// directory under `incoming`, and hash it.
///
/// One channel per file, opened by the daemon and read on a task of
/// its own, so files stream at once: the ask names the file's path,
/// or nothing for a file resource; a channel that ends in the client's
/// error, or without a finish, abandons the whole upload. The id is
/// the one file's hash, or the directory's `dirhash` over every
/// file's.
pub async fn receive(scope: Arc<ScopeHandle>, request: &request::Frame, incoming: &Path) -> Result<Received, Error> {
    match request {
        request::Frame::File { .. } => {
            let file = pieces(scope, &ask(None)?).await;
            ingest(incoming, Kind::File, vec![(Vec::new(), file)]).await
        }
        request::Frame::Directory { files, .. } => {
            let all = paths::validate(files)?;
            let mut streams = Vec::with_capacity(files.len());
            for (path, components) in files.iter().zip(all) {
                streams.push((components, pieces(Arc::clone(&scope), &ask(Some(path.clone()))?).await));
            }
            ingest(incoming, Kind::Directory, streams).await
        }
    }
}

/// The ask for one file's content, as the resources upload frames it.
fn ask(path: Option<String>) -> Result<Vec<u8>, Error> {
    let mut payload = Vec::new();
    channel_request::Frame { path: path.clone() }
        .encode(&mut Writer::new(&mut payload))
        .map_err(|_| Error::Abandoned(path.unwrap_or_default()))?;
    Ok(payload)
}
