//! A daemon connection the proxy announced, re-announced to the
//! caller.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::own::Own;
use super::super::run::Run;
use crate::wire::server::answer::{Answer, answer};

/// The proxy's half, carried to the caller under an id of this end's
/// own.
///
/// The proxy announced a connection its program opened, under an id
/// the proxy minted; this re-announces it to the caller under an id
/// from [`Pairs`](super::super::pairs::Pairs), so that neither end
/// sees the other's numbering, and relays what the caller answers —
/// the daemon's server frames — onto the proxy's channel, in order,
/// until the caller finishes it. The caller's own half arrives
/// separately, as a channel request quoting the id this minted, and
/// [`serve::daemon`](super::super::serve::daemon) answers that one.
///
/// # The id is remembered until the caller takes it
///
/// [`Pairs::open`](super::super::pairs::Pairs::open) remembers which
/// of the proxy's ids this one stands for, in the run's own daemon
/// registry rather than the one Postgres uses;
/// [`take`](super::super::pairs::Pairs::take) is what the caller's
/// half consumes, so a second half for one connection finds nothing
/// and is finished with nothing. A connection the caller never takes
/// is forgotten when this returns.
pub(crate) async fn daemon<R: Runs>(run: Arc<Run>, channel: u32, proxy_id: u32) {
    let caller_id = run.daemons.open(proxy_id);
    let Some(payload) = encoded(&R::Ask::from(Own::Daemon(caller_id))) else {
        run.daemons.forget(caller_id);
        let _ = run.begin.finish(channel).await;
        return;
    };
    let mut half = run.scope.send_channel_request(&payload).await;
    while let Some(bytes) = half.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                if run.begin.respond(channel, &payload).await.is_err() {
                    break;
                }
            }
            Some(Answer::Finish) => break,
            None => {}
        }
    }
    let _ = run.begin.finish(channel).await;
    run.daemons.forget(caller_id);
}
