//! Accepting other daemons' connections through one provider, for
//! the connection's life.

use std::sync::Arc;

use diverge_sdk::provider::endpoints::daemons::accept::client::execute;
use diverge_sdk::wire::client::handle::Handle;

use super::DaemonAcceptor;
use crate::daemon::Daemon;

/// Open the provider protocol's `daemons::accept` on the provider's
/// connection and hold it until it ends, every connection the
/// provider announces taken by the [`DaemonAcceptor`]. A provider that
/// refuses the accept, or does not know it, is one through which no
/// daemon reaches this one. Ended by the provider's connection ending,
/// which ends the scope. The accept is opened once per connection and
/// never again, so the provider's one-at-a-time refusal — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time)
/// — is met only when this daemon's identity accepts there on another
/// connection, and is a refusal like any other: nothing is retried.
pub async fn accept(daemon: Arc<Daemon>, handle: Handle) {
    let acceptor = Arc::new(DaemonAcceptor { daemon });
    let Ok(mut accepting) = execute::execute(&handle, acceptor).await else {
        return;
    };
    accepting.wait().await;
}
