//! Accepting other daemons' connections through one provider, while
//! its record says so.

use std::sync::Arc;

use diverge_sdk::provider::endpoints::daemons::accept::client::execute::{self, Accepting};
use diverge_sdk::wire::client::handle::Handle;

use super::DaemonAcceptor;
use crate::daemon::Daemon;

/// Open the provider protocol's `daemons::accept` on the provider's
/// connection, every connection the provider announces on it taken by
/// the [`DaemonAcceptor`]: the scope, to be held — and cancelled — by
/// whoever asked, which is [`sync`](super::sync), for a provider whose
/// record says `accept_daemons`. `None` for a provider that refuses the
/// accept, or does not know it: one through which no daemon reaches
/// this one. The provider's one-at-a-time refusal — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time)
/// — is met only when this daemon's identity accepts there on another
/// connection, since a sync opens one only with the one before it
/// finished, and is a refusal like any other: nothing is retried here.
/// The scope ends with the provider's connection, if not cancelled
/// before.
pub async fn accept(daemon: &Arc<Daemon>, handle: &Handle) -> Option<Accepting> {
    let acceptor = Arc::new(DaemonAcceptor { daemon: Arc::clone(daemon) });
    execute::execute(handle, acceptor).await.ok()
}
