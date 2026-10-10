//! Accepting other daemons' connections through one provider, for
//! the connection's life.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::daemons::accept::client::execute;
use diverge_sdk::wire::client::handle::Handle;

use super::DaemonAcceptor;
use crate::daemon::Daemon;

/// Open the provider protocol's `daemons::accept` on the provider's
/// connection and hold it until it ends: the identity the provider
/// answers — what the daemon is known by there, which an expose
/// answers and a connector's link names — kept in the provider's slot
/// meanwhile, and every connection the provider announces taken by
/// the [`DaemonAcceptor`]. A provider that refuses the accept, or
/// does not know it, is one through which no daemon reaches this one.
/// Ended by the provider's connection ending, which ends the scope.
pub async fn accept(daemon: Arc<Daemon>, identity: Identity, handle: Handle) {
    let acceptor = Arc::new(DaemonAcceptor {
        daemon: Arc::clone(&daemon),
    });
    let Ok(mut accepting) = execute::execute(&handle, acceptor).await else {
        return;
    };
    daemon.live.set_known_as(&identity, accepting.identity.clone()).await;
    accepting.wait().await;
    daemon.live.clear_known_as(&identity).await;
}
