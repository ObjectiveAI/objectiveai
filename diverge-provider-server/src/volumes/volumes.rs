//! The handler.

use std::pin::Pin;

use diverge_provider_sdk::endpoints::volumes::create::server::response::Creation;
use diverge_provider_sdk::endpoints::volumes::delete::server::response::Deletion;
use diverge_provider_sdk::endpoints::volumes::edit::server::response::Edit;
use diverge_provider_sdk::endpoints::volumes::list::server::response::Volume;
use diverge_provider_sdk::endpoints::volumes::stat::server::response::Stat;
use diverge_provider_sdk::server::volume_manager::VolumeManager;
use diverge_provider_sdk::shared::filetree;
use futures_util::Stream;

use super::Error;

/// The provider's volumes: the directories it offers every identity,
/// and everything done to them.
#[derive(Debug)]
pub struct Volumes;

impl VolumeManager for Volumes {
    type Error = Error;

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

    async fn watch(
        &self,
        _client_identity: &str,
        _name: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<filetree::response::Frame, Error>> + Send>>, Error> {
        unimplemented!()
    }
}
