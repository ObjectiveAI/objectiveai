//! One issuer's key set, held.

use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;

/// The least time between two fetches of one issuer's keys for a `kid`
/// they lack — see the [rules](super), rule 5.
pub(super) const REFETCH: Duration = Duration::from_secs(60);

/// An issuer's key set, when it was fetched, and when it stops being
/// fresh.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    /// The keys, shared with every verification using them.
    pub(super) keys: Arc<JwkSet>,
    /// When they were fetched.
    pub(super) fetched: Instant,
    /// When they stop being fresh.
    pub(super) expires: Instant,
}

impl Entry {
    /// Whether the entry is fresh at `now`.
    pub(super) fn fresh(&self, now: Instant) -> bool {
        now < self.expires
    }

    /// Whether the entry answers for `kid` at `now`: fresh and holding
    /// it, or fresh, lacking it, and fetched too lately to be fetched
    /// again for it.
    pub(super) fn settles(&self, kid: &str, now: Instant) -> bool {
        self.fresh(now) && (self.keys.find(kid).is_some() || now.duration_since(self.fetched) < REFETCH)
    }
}
