//! A file's content as it travels: pieces, then an end or an error.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::provider::endpoints::volumes::write::client::channel_response;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::{Stream, stream};

/// A file's bytes in pieces, each at most a chunk, ending with the
/// file whole or with why it is not — the one shape every file takes
/// on its way through the daemon, whoever sends it and wherever it
/// lands.
pub type Pieces = Pin<Box<dyn Stream<Item = Result<Bytes, String>> + Send>>;

/// One file's content off a channel the daemon opens on a client's
/// scope with `ask` — the family's own ask, naming the file — read as
/// the client answers it: every body a piece, the client's finish the
/// end, the client's error or the channel closing unfinished the one
/// error, after which nothing follows.
pub async fn pieces(scope: Arc<ScopeHandle>, ask: &[u8]) -> Pieces {
    let channel = scope.send_channel_request(ask).await;
    Box::pin(stream::unfold((channel, false), |(mut channel, done)| async move {
        if done {
            return None;
        }
        loop {
            let Some(frame) = channel.response_receiver.recv().await else {
                return Some((Err("the content ended before its finish".to_string()), (channel, true)));
            };
            match ClientFrame::decode(&frame) {
                Ok(ClientFrame::ChannelResponse { payload, .. }) => match channel_response::Frame::decode(payload) {
                    Ok(channel_response::Frame::Body(body)) => {
                        return Some((Ok(Bytes::copy_from_slice(body.0)), (channel, false)));
                    }
                    Ok(channel_response::Frame::Error(error)) => {
                        return Some((Err(format!("the content ended in an error: {}", error.0)), (channel, true)));
                    }
                    Err(error) => return Some((Err(format!("a content frame did not decode: {error}")), (channel, true))),
                },
                Ok(ClientFrame::ChannelResponseFinish { .. }) => return None,
                _ => continue,
            }
        }
    }))
}

/// The bytes at `path` on disk, in pieces of at most `CHUNK_SIZE`.
pub fn from_file(path: std::path::PathBuf) -> Pieces {
    Box::pin(stream::unfold((path, None::<tokio::fs::File>, false), |(path, file, done)| async move {
        if done {
            return None;
        }
        let mut file = match file {
            Some(file) => file,
            None => match tokio::fs::File::open(&path).await {
                Ok(file) => file,
                Err(error) => return Some((Err(format!("{}: {error}", path.display())), (path, None, true))),
            },
        };
        let mut buffer = vec![0u8; diverge_sdk::CHUNK_SIZE];
        match tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await {
            Ok(0) => None,
            Ok(read) => {
                buffer.truncate(read);
                Some((Ok(Bytes::from(buffer)), (path, Some(file), false)))
            }
            Err(error) => Some((Err(format!("{}: {error}", path.display())), (path, None, true))),
        }
    }))
}
