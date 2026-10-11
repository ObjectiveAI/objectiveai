//! The manager that refuses.

use futures_util::{future, stream};

use crate::protocol::context::Context;
use crate::protocol::image_manager;
use crate::protocol::progress::Progress;
use diverge_sdk::provider::endpoints::images::delete::server::response::Deletion;
use diverge_sdk::provider::endpoints::images::list::server::response::Image;
use diverge_sdk::provider::endpoints::images::stat::server::response::Stat;

use super::Error;

/// This provider's image manager: no store, every request refused.
#[derive(Debug, Default)]
pub struct ImageManager;

impl image_manager::ImageManager for ImageManager {
    type Error = Error;

    /// Refused: there is no store to list.
    async fn list(&self, _client_identity: &str) -> Result<Vec<Image>, Error> {
        Err(Error::Unsupported)
    }

    /// Refused: there is no store to look in.
    async fn stat(&self, _client_identity: &str, _name: &str) -> Result<Option<Stat>, Error> {
        Err(Error::Unsupported)
    }

    type Build = stream::Once<future::Ready<Result<Progress, Error>>>;

    /// A build that says nothing and fails at once.
    async fn build(&self, _client_identity: &str, _name: &str, _containerfile: &str, _context: Option<&Context>) -> Self::Build {
        stream::once(future::ready(Err(Error::Unsupported)))
    }

    /// Refused: there is nothing to remove.
    async fn delete(&self, _client_identity: &str, _name: &str) -> Result<Option<Deletion>, Error> {
        Err(Error::Unsupported)
    }
}
