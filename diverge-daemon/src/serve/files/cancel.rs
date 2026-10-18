//! The one channel a client opens on a streaming scope: cancel.

use diverge_sdk::daemon::endpoints::agents::filetree::client::channel_request;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Resolves when the client cancels: every family's cancel is one
/// channel request carrying the one frame `Cancel` — the same byte
/// for agents and tools — read and never answered. The inbox closing
/// is not a cancel.
pub async fn cancelled(scope: &ScopeHandle) {
    loop {
        let Some(bytes) = scope.recv_channel_request().await else {
            return std::future::pending().await;
        };
        if let Ok(ClientFrame::ChannelRequest { payload, .. }) = ClientFrame::decode(&bytes)
            && let Ok(channel_request::Frame::Cancel) = channel_request::Frame::decode(payload)
        {
            return;
        }
    }
}
