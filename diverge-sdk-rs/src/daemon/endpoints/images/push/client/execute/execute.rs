//! Opening the scope, and answering the daemon's asks.

use std::sync::Arc;

use super::super::request;
use super::super::super::server::{channel_request, response};
use crate::daemon::client::stream;
use crate::provider::client::OciStore;
use crate::provider::endpoints::containers::client::Ask;
use crate::provider::endpoints::containers::client::answer;
use crate::shared::containers::request::Image;
use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode as _, Writer};
use crate::wire::frame;

/// An images push that did not open.
pub type OpenError = stream::OpenError<serde_json::Error>;

/// The stream of a push: one [`response::Frame`] per response, ended
/// by the daemon's finish.
pub type ExecuteStream = stream::ExecuteStream<response::Frame, response::FrameError>;

/// Ask the daemon to push the image, answer what it asks of the image
/// from `store`, and hand back the stream of what it answers, without
/// reading anything: zero or more blobs pulled, then the one answer
/// that ends it — see [`response::Frame`] — then the daemon's finish.
/// The rules:
///
/// 1. **The serving task starts before the first frame is read.** A
///    provider that takes the image from the caller asks for its
///    manifest, its blobs and the proof before it answers anything but
///    progress, and the daemon carries each ask here. So a task is
///    spawned right after the request goes out that reads every
///    channel the daemon opens on the scope and answers it from
///    `store`, each ask on a task of its own, exactly as a push put to
///    a provider answers the same asks. It ends when the scope's
///    request stream does, which the router closes with the scope.
/// 2. **A store that holds nothing is a caller with no image.** Every
///    ask is then answered as not held, and the push succeeds only
///    where a registry the provider uses serves the image.
/// 3. **There is no cancel**: a push is not stopped from the client's
///    side, short of the connection ending.
pub async fn execute<O: OciStore + 'static>(handle: &Handle, request: &request::Frame, store: Arc<O>) -> Result<ExecuteStream, OpenError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(stream::OpenError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(stream::OpenError::Send)?;
    let image = Arc::new(request.image.clone());
    tokio::spawn(serve(scope.request_receiver, handle.clone(), scope.scope, image, store));
    Ok(stream::ExecuteStream::new(scope.response_receiver, |_, payload| response::Frame::decode(payload)))
}

/// Read every channel request the daemon opens on `scope` and answer
/// it from `store`, until the scope's request stream ends. `image` is
/// the push's own, which every ask is about.
///
/// A frame that is not a channel request is dropped. An ask that will
/// not decode is a channel this end cannot serve, and is answered as
/// such: the finish with nothing before it.
async fn serve<O: OciStore + 'static>(mut requests: crate::wire::credit::Receiver, handle: Handle, scope: u32, image: Arc<Image>, store: Arc<O>) {
    while let Some(bytes) = requests.recv().await {
        let Ok(frame::server::ServerFrame::ChannelRequest { channel, payload, .. }) = frame::server::ServerFrame::decode(&bytes) else {
            continue;
        };
        let ask = match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::OciManifest(request)) => Ask::OciManifest(request.digest),
            Ok(channel_request::Frame::OciBlob(request)) => Ask::OciBlob(request.digest),
            Ok(channel_request::Frame::OciProve(request)) => Ask::OciProve(request),
            Err(_) => {
                let handle = handle.clone();
                tokio::spawn(async move {
                    let _ = handle.send_channel_response_finish(scope, channel).await;
                });
                continue;
            }
        };
        tokio::spawn(answer::answer_oci(handle.clone(), scope, channel, ask, Arc::clone(&image), Arc::clone(&store)));
    }
}
