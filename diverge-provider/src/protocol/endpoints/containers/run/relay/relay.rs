//! Every ask on the begin scope, dispatched.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Runs;
use super::super::run::Run;
use super::{daemon, one, postgres, stream};
use diverge_sdk::container_proxy::outside::client::{Ask, Asks};

/// Read the proxy's asks off the begin scope for as long as it lives,
/// each on a task of its own; when they end, the container is gone.
pub(crate) async fn relay<R: Runs>(run: Arc<Run>, mut asks: Asks<Ask>) {
    while let Some((channel, ask)) = asks.next().await {
        match ask {
            // An ask this end cannot read is a channel it cannot
            // serve: the finish with nothing before it.
            None => {
                let _ = run.begin.finish(channel).await;
            }
            Some(ask) => run.spawn(answer::<R>(Arc::clone(&run), channel, ask)).await,
        }
    }
    run.over.notify_one();
}

/// One ask, by its kind: a database connection and a daemon
/// connection are each a pair, a notification stream is a stream, and
/// the rest answer once.
async fn answer<R: Runs>(run: Arc<Run>, channel: u32, ask: Ask) {
    match ask {
        Ask::Daemon(proxy_id) => daemon::daemon::<R>(run, channel, proxy_id).await,
        Ask::Postgres(proxy_id) => postgres::postgres::<R>(run, channel, proxy_id).await,
        Ask::McpNotifications => stream::stream::<R>(run, channel, &ask).await,
        _ => one::one::<R>(run, channel, &ask).await,
    }
}
