//! The handler.

use diverge_provider_sdk::endpoints::volumes::create::server::response::Creation;
use diverge_provider_sdk::endpoints::volumes::delete::server::response::Deletion;
use diverge_provider_sdk::endpoints::volumes::edit::server::response::Edit;
use diverge_provider_sdk::endpoints::volumes::list::server::response::Volume;
use diverge_provider_sdk::endpoints::volumes::stat::server::response::Stat;
use diverge_provider_sdk::server::volume_manager;
use diverge_provider_sdk::shared::filetree;
use std::sync::Arc;

use futures_util::stream;

use super::{Cache, Error, Identity};
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
    /// Nothing yet: the stream a watch hands back arrives with the
    /// implementation.
    type Watch = stream::Empty<Result<filetree::response::Frame, Error>>;

    async fn list(&self, _client_identity: &str) -> Result<Vec<Volume>, Error> {
        unimplemented!()
    }

    async fn stat(&self, _client_identity: &str, _name: &str) -> Result<Stat, Error> {
        unimplemented!()
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

    async fn edit(&self, _client_identity: &str, _name: &str, _bytes: u64) -> Result<Edit, Error> {
        unimplemented!()
    }

    async fn delete(&self, _client_identity: &str, _name: &str) -> Result<Deletion, Error> {
        unimplemented!()
    }

    async fn watch(&self, _client_identity: &str, _name: &str) -> Result<Self::Watch, Error> {
        unimplemented!()
    }
}
