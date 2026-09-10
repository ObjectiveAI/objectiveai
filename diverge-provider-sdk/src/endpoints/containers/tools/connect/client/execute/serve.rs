//! The loop that answers the one ask a provider makes of a connector.

use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::client::handle::Handle;
use crate::decode::Decode as _;
use crate::endpoints::containers::client::{Encoders, Writes, answer};
use crate::endpoints::containers::tools::connect::server;
use crate::frame;

/// Read every channel request the provider opens on the connect
/// scope and answer it: each is a write's content, served from the
/// pending writes on a task of its own. Anything else on the stream
/// is dropped, unanswered.
pub(super) async fn serve(
    mut requests: UnboundedReceiver<Bytes>,
    handle: Handle,
    scope: u32,
    writes: Arc<Writes>,
    encoders: Encoders,
) {
    while let Some(bytes) = requests.recv().await {
        let Ok(frame::server::ServerFrame::ChannelRequest { channel, payload, .. }) =
            frame::server::ServerFrame::decode(&bytes)
        else {
            continue;
        };
        let Ok(server::channel_request::Frame(request)) = server::channel_request::Frame::decode(payload)
        else {
            continue;
        };
        let handle = handle.clone();
        let writes = Arc::clone(&writes);
        tokio::spawn(async move {
            let _ = answer::write_content(&handle, scope, channel, request.write_id, writes, encoders).await;
        });
    }
}
