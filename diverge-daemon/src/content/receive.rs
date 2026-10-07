//! An upload, read off the channels the daemon opens.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::resources::Kind;
use diverge_sdk::daemon::endpoints::resources::upload::client::request;
use diverge_sdk::daemon::endpoints::resources::upload::server::channel_request;
use diverge_sdk::provider::endpoints::volumes::write::client::channel_response;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use sha2::{Digest as _, Sha256};
use tokio::io::AsyncWriteExt as _;
use tokio::task::JoinSet;

use super::{Error, hash_of, paths};
use crate::store::hash;

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
/// One channel per file, opened by the daemon and answered on a task
/// of its own, so files stream at once: the ask names the file's path,
/// or nothing for a file resource; every body piece is appended to
/// the file and folded into its hash; the client's finish ends the
/// file, and a zero-byte file is one empty piece. A channel that ends
/// in the client's error, or without a finish, abandons the whole
/// upload, and the incoming directory is removed. The id is the one
/// file's hash, or the directory's `dirhash` over every file's.
pub async fn receive(scope: Arc<ScopeHandle>, request: &request::Frame, incoming: &Path) -> Result<Received, Error> {
    let fresh = incoming.join(uuid::Uuid::new_v4().to_string());
    let outcome = receive_into(scope, request, &fresh).await;
    if outcome.is_err() {
        let _ = tokio::fs::remove_dir_all(&fresh).await;
    }
    outcome
}

/// The upload into `fresh`, which does not exist yet.
async fn receive_into(scope: Arc<ScopeHandle>, request: &request::Frame, fresh: &Path) -> Result<Received, Error> {
    match request {
        request::Frame::File { .. } => {
            tokio::fs::create_dir_all(fresh).await.map_err(|source| Error::Io {
                path: fresh.to_path_buf(),
                source,
            })?;
            let file = fresh.join("file");
            let (digest, bytes) = content(scope, None, file).await?;
            Ok(Received {
                incoming: fresh.join("file"),
                kind: Kind::File,
                id: hash::file_id(&digest),
                bytes,
            })
        }
        request::Frame::Directory { files, .. } => {
            let all = paths::validate(files)?;
            let mut receiving = JoinSet::new();
            for (path, components) in files.iter().zip(all) {
                let at = paths::join(fresh, &components);
                if let Some(parent) = at.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|source| Error::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
                let scope = Arc::clone(&scope);
                let path = path.clone();
                receiving.spawn(async move {
                    let received = content(scope, Some(path.clone()), at).await;
                    received.map(|(digest, bytes)| (path, digest, bytes))
                });
            }
            let mut hashed = Vec::with_capacity(files.len());
            let mut total = 0;
            while let Some(joined) = receiving.join_next().await {
                let (path, digest, bytes) = joined.map_err(|_| Error::Abandoned(String::new()))??;
                total += bytes;
                hashed.push((path, hash::file_id(&digest)));
            }
            Ok(Received {
                incoming: fresh.to_path_buf(),
                kind: Kind::Directory,
                id: hash::directory_id(&hashed),
                bytes: total,
            })
        }
    }
}

/// One file's content: the channel opened with its ask, every piece
/// written to `at` and hashed, to the finish.
async fn content(scope: Arc<ScopeHandle>, path: Option<String>, at: PathBuf) -> Result<(Vec<u8>, u64), Error> {
    let ask = channel_request::Frame { path: path.clone() };
    let mut payload = Vec::new();
    ask.encode(&mut Writer::new(&mut payload))
        .map_err(|_| Error::Abandoned(path.clone().unwrap_or_default()))?;
    let mut channel = scope.send_channel_request(&payload).await;
    let mut file = tokio::fs::File::create(&at).await.map_err(|source| Error::Io {
        path: at.clone(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut bytes = 0;
    let named = path.unwrap_or_default();
    loop {
        let Some(frame) = channel.response_receiver.recv().await else {
            return Err(Error::Abandoned(named));
        };
        match ClientFrame::decode(&frame) {
            Ok(ClientFrame::ChannelResponse { payload, .. }) => match channel_response::Frame::decode(payload) {
                Ok(channel_response::Frame::Body(body)) => {
                    file.write_all(body.0).await.map_err(|source| Error::Io {
                        path: at.clone(),
                        source,
                    })?;
                    hasher.update(body.0);
                    bytes += body.0.len() as u64;
                }
                Ok(channel_response::Frame::Error(_)) => return Err(Error::Abandoned(named)),
                Err(error) => return Err(Error::Channel(error)),
            },
            Ok(ClientFrame::ChannelResponseFinish { .. }) => break,
            _ => {}
        }
    }
    file.flush().await.map_err(|source| Error::Io { path: at, source })?;
    Ok((hash_of(hasher), bytes))
}
