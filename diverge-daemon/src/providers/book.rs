//! The providers' records, as every connection reads them.

use std::net::IpAddr;
use std::sync::Arc;

use arc_swap::ArcSwap;
use diverge_sdk::shared::Identity;

use crate::store::{self, Store, providers};

/// Every provider's record as it stands, read without waiting: what the
/// handshake, the dials, a list and a run's choice of provider all
/// read. Loaded at the start and loaded again after every committed
/// change, by whoever made it — see
/// [`reconcile`](super::reconcile). The rules:
///
/// 1. **A record.** A provider is the daemon's when it has a record,
///    enabled or not: a run may be pinned to it, its volumes listed,
///    its status reported.
/// 2. **Enabled and in its direction.** A connection is admitted only
///    for an enabled record, in the record's direction — see
///    [`admits`](Self::admits) and [`dials`](Self::dials).
#[derive(Debug)]
pub struct ProviderBook {
    records: ArcSwap<Vec<providers::Record>>,
}

impl ProviderBook {
    /// The records as `store` keeps them now.
    pub async fn load(store: &Store) -> Result<ProviderBook, store::Error> {
        let mut conn = store.acquire().await?;
        let records = providers::all(&mut conn).await?;
        Ok(ProviderBook { records: ArcSwap::from_pointee(records) })
    }

    /// Read the records again, after a change was committed.
    pub async fn reload(&self, store: &Store) -> Result<(), store::Error> {
        let mut conn = store.acquire().await?;
        let records = providers::all(&mut conn).await?;
        self.records.store(Arc::new(records));
        Ok(())
    }

    /// Every record, oldest first.
    pub fn all(&self) -> Arc<Vec<providers::Record>> {
        self.records.load_full()
    }

    /// The record of `identity`, if there is one.
    pub fn record(&self, identity: &Identity) -> Option<providers::Record> {
        self.records.load().iter().find(|record| &record.identity == identity).cloned()
    }

    /// Every provider's identity, oldest record first: the order the
    /// daemon tries them in when nothing pins a run.
    pub fn identities(&self) -> Vec<Identity> {
        self.records.load().iter().map(|record| record.identity.clone()).collect()
    }

    /// Whether `identity` has a record — rule 1.
    pub fn accepts(&self, identity: &Identity) -> bool {
        self.records.load().iter().any(|record| &record.identity == identity)
    }

    /// Whether a provider dialling in as `identity` from `ip` is
    /// admitted — rule 2: an enabled, incoming record whose addresses
    /// contain the source, or name none.
    pub fn admits(&self, identity: &Identity, ip: IpAddr) -> bool {
        self.records.load().iter().any(|record| &record.identity == identity && record.enabled && record.direction.admits(ip))
    }

    /// The address to dial `identity` at, when its record is enabled
    /// and outgoing — rule 2.
    pub fn dials(&self, identity: &Identity) -> Option<String> {
        let records = self.records.load();
        let record = records.iter().find(|record| &record.identity == identity && record.enabled)?;
        record.direction.dialled().map(str::to_string)
    }

    /// Whether the daemon is discoverable to `identity`.
    pub fn discover(&self, identity: &Identity) -> bool {
        self.records.load().iter().any(|record| &record.identity == identity && record.enabled && record.discover)
    }
}
