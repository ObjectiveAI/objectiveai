//! What every request is served with.

use tokio::sync::watch;

use crate::config::Config;
use crate::store::Store;

/// The issuer's state, shared by every request: the document it was
/// started on, the database, and the stop.
#[derive(Debug)]
pub struct Issuer {
    /// `issuer.yaml`, as it was read at the start.
    pub config: Config,
    /// `issuer.sqlite`.
    pub store: Store,
    /// Sent `true` once, to stop the issuer — see [`run`](super::run).
    pub stop: watch::Sender<bool>,
}
