//! The cache.

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::RwLock;

use super::VerifyError;
use super::entry::Entry;

/// Every issuer's key set, as far as it has been fetched: the cache a
/// verification takes — see the [rules](super).
///
/// One entry per issuer, each behind its own [`RwLock`], apart from
/// the map: many verifications read an entry at once, and a refresh
/// holds only its own issuer's. Cloning it shares the entries and the
/// HTTP client.
#[derive(Debug, Clone)]
pub struct KeySets {
    pub(super) entries: Arc<DashMap<String, Arc<RwLock<Option<Entry>>>>>,
    pub(super) http: reqwest::Client,
    pub(super) lifetime: Duration,
}

impl KeySets {
    /// An empty cache, whose key sets are held for `lifetime` where the
    /// issuer's answer names no `max-age`. The HTTP client follows no
    /// redirect, so no issuer's answer can send a fetch elsewhere.
    pub fn new(lifetime: Duration) -> Result<KeySets, VerifyError> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| VerifyError::Fetch(error.to_string()))?;
        Ok(KeySets { entries: Arc::new(DashMap::new()), http, lifetime })
    }

    /// The issuer's entry, made empty where there is none. The map's
    /// guard is let go before this returns, so it is never held across
    /// an await.
    pub(super) fn entry(&self, issuer: &str) -> Arc<RwLock<Option<Entry>>> {
        if let Some(entry) = self.entries.get(issuer) {
            return Arc::clone(entry.value());
        }
        Arc::clone(self.entries.entry(issuer.to_string()).or_insert_with(|| Arc::new(RwLock::new(None))).value())
    }
}
