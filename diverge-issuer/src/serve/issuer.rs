//! What every request is served with.

use tokio::sync::watch;

use crate::config::Config;
use crate::signing::Signing;
use crate::store::Store;

/// The issuer's state, shared by every request: the document it was
/// started on, the database, the signing keys, the two documents made
/// from them at the start, and the stop.
pub struct Issuer {
    /// `issuer.yaml`, as it was read at the start.
    pub config: Config,
    /// `issuer.sqlite`.
    pub store: Store,
    /// The signing keys, read at the start.
    pub signing: Signing,
    /// The discovery document, as JSON.
    pub discovery: String,
    /// The key set, as JSON.
    pub key_set: String,
    /// Sent `true` once, to stop the issuer — see [`run`](super::run).
    pub stop: watch::Sender<bool>,
}
