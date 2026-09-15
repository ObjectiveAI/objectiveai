//! The handler.

use std::sync::Arc;

use diverge_provider_sdk::endpoints::volumes::create::server::response::Creation;
use diverge_provider_sdk::endpoints::volumes::list::server::response::Volume;
use diverge_provider_sdk::server::volume_manager;

use super::{Cache, Error, Handle, Identity};
use crate::config::volumes::{Fixed, Store};

/// The provider's volumes: the directories it offers every identity,
/// and everything done to them, over the stores it may create in and
/// the fixed volumes it holds already.
#[derive(Debug)]
pub struct VolumeManager {
    /// Where volumes may be created, as the `volumes` section names
    /// them. `None` is a provider that creates none.
    pub stores: Option<Vec<Store>>,
    /// The volumes that exist already, as the `volumes` section names
    /// them. `None` is a provider that holds none.
    pub fixed: Option<Vec<Fixed>>,
    /// What is known about the volumes between calls: every identity
    /// that has asked, its volumes by name, and what a walk found.
    pub cache: Cache,
}

impl VolumeManager {
    /// The identity's volumes, read from the stores and the fixed
    /// list the first time the identity is named and held from then
    /// on. Every method that names a volume starts here.
    pub async fn identity(&self, client_identity: &str) -> Arc<Identity> {
        self.cache
            .identity(
                self.stores.as_deref().unwrap_or_default(),
                self.fixed.as_deref().unwrap_or_default(),
                client_identity,
            )
            .await
    }
}

impl volume_manager::VolumeManager for VolumeManager {
    type Error = Error;
    type Volume = Handle;

    async fn list(&self, _client_identity: &str) -> Result<Vec<Volume>, Error> {
        unimplemented!()
    }

    /// The cache's entry under `name` for this identity, or `None`
    /// where it has none. Never fails: the cache reads nothing to
    /// answer.
    async fn get(&self, client_identity: &str, name: &str) -> Result<Option<Handle>, Error> {
        Ok(self.identity(client_identity).await.volume(name).map(Handle::new))
    }

    async fn create_capacity(&self, _client_identity: &str) -> Result<u64, Error> {
        unimplemented!()
    }

    async fn create(&self, _client_identity: &str, _name: &str, _bytes: u64) -> Result<Creation, Error> {
        unimplemented!()
    }

    async fn edit_capacity(&self, _client_identity: &str, _name: &str) -> Result<u64, Error> {
        unimplemented!()
    }

    async fn delete(&self, _client_identity: &str, _name: &str) -> Result<(), Error> {
        unimplemented!()
    }
}
