//! Opening the scope, and answering the provider's asks.

use std::sync::Arc;

use super::super::request;
use super::super::super::server::channel_request;
use super::{ExecuteError, ExecuteStream};
use crate::provider::client::OciStore;
use crate::provider::endpoints::containers::client::Ask;
use crate::provider::endpoints::containers::client::answer;
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Ask the provider to push the image, answer what it asks of the
/// image from `store`, and read what the push says.
///
/// # The serving task starts before the first frame is read
///
/// A provider that takes the image from the caller asks for its
/// manifest, its blobs and the proof before it answers anything but
/// progress, so a task is spawned right after the request goes out
/// that reads every channel the provider opens on the scope and
/// answers it from `store`, each ask on a task of its own, exactly as
/// a run answers the same asks. It ends when the scope's request
/// stream does, which the router closes with the scope.
///
/// Reads nothing before returning: the provider's first frame is a
/// blob pulled, or the push's result, and all of them are the
/// stream's. The stream ends at the provider's finish, which follows
/// the result. A caller opens no channel on a push, so a push cannot
/// be stopped from the caller's side, short of the connection ending.
pub async fn execute<O: OciStore + 'static>(
    handle: &Handle,
    request: &request::Frame,
    store: Arc<O>,
) -> Result<ExecuteStream, ExecuteError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    tokio::spawn(serve(scope.request_receiver, handle.clone(), scope.scope, store));
    Ok(ExecuteStream::new(scope.response_receiver))
}

/// Read every channel request the provider opens on `scope` and
/// answer it from `store`, until the scope's request stream ends.
///
/// A frame that is not a channel request is dropped. An ask that will
/// not decode is a channel this end cannot serve, and is answered as
/// such: the finish with nothing before it.
async fn serve<O: OciStore + 'static>(
    mut requests: crate::wire::credit::Receiver,
    handle: Handle,
    scope: u32,
    store: Arc<O>,
) {
    while let Some(bytes) = requests.recv().await {
        let Ok(frame::server::ServerFrame::ChannelRequest { channel, payload, .. }) =
            frame::server::ServerFrame::decode(&bytes)
        else {
            continue;
        };
        let ask = match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::OciManifest(request)) => Ask::OciManifest(request.digest),
            Ok(channel_request::Frame::OciBlob(request)) => Ask::OciBlob(request.digest),
            Ok(channel_request::Frame::OciProve(request)) => Ask::OciProve {
                digest: request.digest,
                platform: request.platform,
                nonce: request.nonce,
            },
            Err(_) => {
                let handle = handle.clone();
                tokio::spawn(async move {
                    let _ = handle.send_channel_response_finish(scope, channel).await;
                });
                continue;
            }
        };
        tokio::spawn(answer::answer_oci(handle.clone(), scope, channel, ask, Arc::clone(&store)));
    }
}
