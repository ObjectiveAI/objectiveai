//! A mount's asks, carried to the caller with the mount's id.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::run::Run;
use super::super::setup::Mount;
use crate::container_proxy::outside::endpoints::fuse::mount::client::execute::{Ask, ExecuteHandle};
use crate::wire::server::answer::{Answer, answer};

/// Read the mount's asks off its scope for as long as it lives, each
/// answered once on a task of its own. The scope is the mount, so the
/// ask carries no id; the caller's is put back in front of it here.
pub(crate) async fn fuse<R: Runs>(run: Arc<Run>, mount: Mount) {
    let Mount { id, handle, mut asks } = mount;
    let id: Arc<str> = id.into();
    while let Some((channel, ask)) = asks.next().await {
        match ask {
            None => {
                let _ = handle.finish(channel).await;
            }
            Some(ask) => {
                run.spawn(one::<R>(Arc::clone(&run), Arc::clone(&id), handle.clone(), channel, ask))
                    .await;
            }
        }
    }
}

/// One ask: the caller's one answer onto the mount's channel, then
/// the finish — or the finish alone for a caller that could not
/// serve it, or is gone.
async fn one<R: Runs>(run: Arc<Run>, id: Arc<str>, handle: ExecuteHandle, channel: u32, ask: Ask) {
    // Encoded before the first await: the family's frame borrows the
    // ask and is not the future's to hold.
    let payload = encoded(&R::fuse(&id, &ask));
    let first = match payload {
        Some(payload) => {
            let mut caller = run.scope.send_channel_request(&payload).await;
            let mut first = None;
            while let Some(bytes) = caller.response_receiver.recv().await {
                match answer(&bytes) {
                    Some(Answer::Frame(payload)) => {
                        if first.is_none() {
                            first = Some(payload);
                        }
                    }
                    Some(Answer::Finish) => break,
                    None => {}
                }
            }
            first
        }
        None => None,
    };
    if let Some(bytes) = first {
        let _ = handle.respond(channel, &bytes).await;
    }
    let _ = handle.finish(channel).await;
}
