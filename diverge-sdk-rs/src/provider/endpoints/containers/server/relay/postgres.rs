//! A database connection the container opened: a pair on each wire.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::own::Own;
use super::super::run::Run;
use crate::wire::server::answer::{Answer, answer};

/// The proxy announced a connection under `proxy_id` and holds the
/// driver's bytes. This end mints an id for the caller, remembers the
/// proxy's under it, and opens the provider's half on the run scope:
/// everything the database says comes back on it and goes onto the
/// proxy's channel, until the caller finishes — or declines with the
/// empty finish. The caller's own half, quoting this end's id, is
/// what opens this end's half on the begin scope, in `serve`.
pub(crate) async fn postgres<R: Runs>(run: Arc<Run>, channel: u32, proxy_id: u32) {
    let caller_id = run.pairs.open(proxy_id);
    let Some(payload) = encoded(&R::Ask::from(Own::Postgres(caller_id))) else {
        run.pairs.forget(caller_id);
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
    run.pairs.forget(caller_id);
}
