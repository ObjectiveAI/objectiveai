//! What the manager hands the SDK for one volume.

use std::sync::Arc;

use diverge_provider_sdk::endpoints::volumes::edit::server::response::Edit;
use diverge_provider_sdk::endpoints::volumes::stat::server::response::Stat;
use diverge_provider_sdk::server::volume;
use diverge_provider_sdk::shared::filetree;
use futures_util::stream;

use super::{Error, Volume};

/// The SDK's `Volume`: the cache's entry for one volume, shared with
/// the cache, so what the SDK locks is what the next listing reads.
///
/// Its own type rather than the `Arc` itself, because the trait is
/// the SDK's and the `Arc` is the standard library's, and neither
/// crate is this one. Everything it does, it does on the entry.
#[derive(Debug, Clone)]
pub struct Handle {
    volume: Arc<Volume>,
}

impl Handle {
    pub fn new(volume: Arc<Volume>) -> Self {
        Handle { volume }
    }

    /// The cache's entry.
    pub fn volume(&self) -> &Arc<Volume> {
        &self.volume
    }
}

impl volume::Volume for Handle {
    type Error = Error;
    /// Nothing yet: the stream a watch hands back arrives with the
    /// implementation.
    type Watch = stream::Empty<Result<filetree::response::Frame, Error>>;

    /// The entry's flag, set if it was clear. Never fails: a flag in
    /// memory always answers.
    async fn lock(&self) -> Result<bool, Error> {
        Ok(self.volume.lock().await)
    }

    async fn unlock(&self) -> Result<(), Error> {
        self.volume.unlock().await;
        Ok(())
    }

    async fn locked(&self) -> Result<bool, Error> {
        Ok(self.volume.locked().await)
    }

    async fn stat(&self) -> Result<Stat, Error> {
        unimplemented!()
    }

    async fn edit(&self, _bytes: u64) -> Result<Edit, Error> {
        unimplemented!()
    }

    async fn watch(&self) -> Result<Self::Watch, Error> {
        unimplemented!()
    }
}
