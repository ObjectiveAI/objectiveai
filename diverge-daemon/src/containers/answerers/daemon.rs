//! The container's `/daemon` connections, served as its account or
//! under its template's grants.

use std::sync::Arc;

use diverge_sdk::provider::client::Daemon as DaemonAnswerer;
use diverge_sdk::shared::containers::daemon as pair;
use tokio::sync::mpsc::UnboundedReceiver;

use super::Answerer;
use crate::judge::Who;
use crate::serve::{self, PairFrames};

/// A connection the container's program opened, taken when the
/// container has a standing — a record's account, read fresh for
/// every request, or a dependency's grants, fixed at its deploy — and
/// declined when it has none: served by [`serve::serve_pair`], the
/// same session and dispatch a socket gets, for that standing — no
/// credential passes. Every frame the program sends is a use of the
/// container.
impl DaemonAnswerer for Answerer {
    type Frames = PairFrames;

    async fn connect(&self, _: u32, from_program: UnboundedReceiver<pair::client::Owned>) -> Option<Self::Frames> {
        let who = match (&self.standing, self.account) {
            (Some(standing), _) => Who::Dependency(Arc::clone(standing)),
            (None, Some(account)) => Who::Account(account),
            (None, None) => return None,
        };
        Some(serve::serve_pair(Arc::clone(&self.daemon), who, from_program, Some(self.touched.clone())))
    }
}
