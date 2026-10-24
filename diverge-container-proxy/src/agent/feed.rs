//! The secrets, fed: the server's secrets channel, answered once the
//! program has applied them.

use std::sync::Arc;

use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::containers::secrets::feed::response;
use tokio::sync::oneshot;

use super::Cmd;
use crate::encode::encoded;
use crate::proxy::Proxy;
use crate::reply::reply;

/// Serve one secrets channel: the values go to the driver, which
/// feeds them to the program and opens the gate when the program has
/// applied them, and the channel is answered `Fed` then — or `Error`,
/// the program's refusal in its own words, the gate as it was — one
/// frame, then the finish. Nothing times it out: the program holds
/// the feed until the values are its own. A driver that is gone, or
/// one that never was, is the finish with nothing before it.
pub async fn feed(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32, values: Values) {
    let (applied, heard) = oneshot::channel();
    let sent = proxy
        .commands()
        .is_some_and(|commands| commands.send(Cmd::Feed { values, reply: applied }).is_ok());
    if !sent {
        reply(&scope, channel, None).await;
        return;
    }
    let payload = match heard.await {
        Ok(Ok(())) => encoded(&response::Frame::Fed),
        Ok(Err(error)) => encoded(&response::Frame::Error(error)),
        Err(_) => None,
    };
    reply(&scope, channel, payload).await;
}
