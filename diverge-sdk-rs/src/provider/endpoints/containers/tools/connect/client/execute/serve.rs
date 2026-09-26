//! The loop that answers the one ask a provider makes of a connector.

use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::wire::client::handle::Handle;
use crate::wire::decode::Decode as _;
use crate::provider::endpoints::containers::client::{Encoders, Writes, answer};
use crate::provider::endpoints::containers::tools::connect::server;
use crate::wire::frame;

/// Read every channel request the provider opens on the connect
/// scope and answer it: each is a write's content, served from the
/// pending writes on a task of its own. A frame that is not a channel
/// request is dropped — there is no channel to answer. A request that
/// is not a write's content ask is a channel this end cannot serve,
/// and is answered as such: the finish with nothing before it.
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
            let handle = handle.clone();
            tokio::spawn(async move {
                let _ = handle.send_channel_response_finish(scope, channel).await;
            });
            continue;
        };
        let handle = handle.clone();
        let writes = Arc::clone(&writes);
        tokio::spawn(async move {
            let _ = answer::write_content(&handle, scope, channel, request.write_id, writes, encoders).await;
        });
    }
}
