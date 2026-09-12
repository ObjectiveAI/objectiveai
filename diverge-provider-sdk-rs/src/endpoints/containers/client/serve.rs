//! The loop that reads a run scope's server-opened channels and
//! answers each.

use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

use super::answer;
use super::{Ask, Encoders, Writes};
use crate::client::handle::Handle;
use crate::client::{
    Answerers, CommandRunner, ConnectionAuthorizer, FuseServer, IdentityStore, McpServer, OciStore,
    PostgresDialer, Vault,
};
use crate::frame;

/// Read every channel request the provider opens on `scope` and
/// answer it on a task of its own, until the scope's request stream
/// ends.
///
/// `decode` is the family's: its `server::channel_request::Frame`
/// read into the one owned [`Ask`]. A frame that is not a channel
/// request, or an ask that will not decode, is dropped — the wire
/// says nothing else can be on this stream, and a provider that sent
/// it anyway gets no answer, which it reads as the channel dying.
/// Each answer runs on its own task so several can be in flight at
/// once: answers may be given in any order, and an agent making
/// several calls at once is the ordinary case.
pub(crate) async fn serve<O, A, I, P, C, V, M, F>(
    mut requests: UnboundedReceiver<Bytes>,
    handle: Handle,
    scope: u32,
    writes: Arc<Writes>,
    answerers: Answerers<O, A, I, P, C, V, M, F>,
    decode: fn(&[u8]) -> Option<Ask>,
    encoders: Encoders,
) where
    O: OciStore + 'static,
    A: ConnectionAuthorizer + 'static,
    I: IdentityStore + 'static,
    P: PostgresDialer + 'static,
    C: CommandRunner + 'static,
    V: Vault + 'static,
    M: McpServer + 'static,
    F: FuseServer + 'static,
{
    while let Some(bytes) = requests.recv().await {
        let Ok(frame::server::ServerFrame::ChannelRequest { channel, payload, .. }) =
            frame::server::ServerFrame::decode(&bytes)
        else {
            continue;
        };
        let Some(ask) = decode(payload) else {
            continue;
        };
        tokio::spawn(answer::answer(
            handle.clone(),
            scope,
            channel,
            ask,
            Arc::clone(&writes),
            answerers.clone(),
            encoders,
        ));
    }
}
