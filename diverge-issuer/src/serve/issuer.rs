//! What every request is served with.

use tokio::sync::{RwLock, watch};

use crate::config::Config;
use crate::signing::Live;
use crate::store::Store;

/// The issuer's state, shared by every request: the document it was
/// started on, the database, the live signing keys, the discovery
/// document, and the stop.
pub struct Issuer {
    /// `issuer.yaml`, as it was read at the start.
    pub config: Config,
    /// `issuer.sqlite`.
    pub store: Store,
    /// The signing keys and the key set made of them, read at the start
    /// and swapped whole by a rotation. A request that signs, verifies
    /// or publishes holds the read lock for its own work; a rotation
    /// holds the write lock, so no token is signed with a key half
    /// replaced.
    pub keys: RwLock<Live>,
    /// The discovery document, as JSON.
    pub discovery: String,
    /// Sent `true` once, to stop the issuer — see [`run`](super::run).
    pub stop: watch::Sender<bool>,
}
