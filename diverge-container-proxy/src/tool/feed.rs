//! The secrets, fed: the server's secrets channel, answered once the
//! program has applied them.

use std::sync::Arc;

use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::containers::secrets::feed::response;

use crate::encode::encoded;
use crate::program;
use crate::proxy::Proxy;
use crate::reply::reply;

/// Serve one secrets channel: the values are fed to the program's
/// server, held until it has applied them; then the gate opens and
/// the channel is answered `Fed` — or `Error`, the program's refusal
/// in its own words, the gate as it was — one frame, then the finish.
/// The gate opens before the word goes out, so an exchange waiting
/// behind it is on its way by the time the server hears `Fed`.
pub async fn feed(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32, values: Values) {
    let frame = match program::feed(&proxy.upstream, values).await {
        Ok(()) => {
            proxy.activity.open();
            response::Frame::Fed
        }
        Err(error) => response::Frame::Error(error),
    };
    reply(&scope, channel, encoded(&frame)).await;
}
