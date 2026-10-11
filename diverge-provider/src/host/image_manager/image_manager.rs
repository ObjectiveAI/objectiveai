//! The manager over the store.

use std::sync::Arc;

use futures_util::{future, stream};

use super::{Build, Error, Store, build};
use crate::host::container_deployer::ContainerDeployer;
use crate::host::tools::podman;
use crate::host::volume_manager::VolumeManager;
use crate::protocol::context::Context;
use crate::protocol::image_manager;
use crate::protocol::progress::Progress;
use diverge_sdk::provider::endpoints::images::delete::server::response::Deletion;
use diverge_sdk::provider::endpoints::images::list::server::response::Image;
use diverge_sdk::provider::endpoints::images::stat::server::response::Stat;

/// This provider's image manager: the store, the volumes a build
/// reads its context from, and the deployer a build and a delete
/// reach the cache through.
#[derive(Debug)]
pub struct ImageManager {
    /// The store; `None` where the configuration has none.
    store: Option<Arc<Store>>,
    /// The volumes, for a build's context.
    volumes: Arc<VolumeManager>,
    /// The deployer: its mounts directory and auth file for a build,
    /// its cache for a delete's in-use answer.
    deployer: Arc<ContainerDeployer>,
}

impl ImageManager {
    /// A manager over `store`, with the volumes and the deployer.
    pub fn new(store: Option<Arc<Store>>, volumes: Arc<VolumeManager>, deployer: Arc<ContainerDeployer>) -> Self {
        ImageManager { store, volumes, deployer }
    }
}

impl image_manager::ImageManager for ImageManager {
    type Error = Error;

    /// The identity's records; none with no store.
    async fn list(&self, client_identity: &str) -> Result<Vec<Image>, Error> {
        match &self.store {
            Some(store) => store.list(client_identity).await,
            None => Ok(Vec::new()),
        }
    }

    /// The identity's record under `name`, with its Containerfile.
    async fn stat(&self, client_identity: &str, name: &str) -> Result<Option<Stat>, Error> {
        let Some(store) = &self.store else {
            return Ok(None);
        };
        let Some(meta) = store.read(client_identity, name).await? else {
            return Ok(None);
        };
        Ok(Some(Stat {
            image: Image {
                name: name.to_string(),
                digest: meta.digest,
                bytes: meta.bytes,
                created: meta.created,
            },
            containerfile: meta.containerfile,
        }))
    }

    type Build = Build;

    /// The build, on a task of its own; with no store, the one error.
    async fn build(&self, client_identity: &str, name: &str, containerfile: &str, context: Option<&Context>) -> Self::Build {
        match &self.store {
            Some(store) => build(
                Arc::clone(store),
                Arc::clone(&self.volumes),
                Arc::clone(&self.deployer),
                client_identity,
                name,
                containerfile,
                context,
            ),
            None => Box::pin(stream::once(future::ready(Err::<Progress, Error>(Error::Absent)))),
        }
    }

    /// The image removed from the root and its record removed, under
    /// the identity's lock; in use where a container runs the copy in
    /// the cache; the copy removed from the cache too, where it can
    /// be. `None` for a name the identity has no image by.
    async fn delete(&self, client_identity: &str, name: &str) -> Result<Option<Deletion>, Error> {
        let Some(store) = &self.store else {
            return Ok(None);
        };
        let namespace = store.namespace(client_identity);
        let _namespace = namespace.lock().await;
        if store.read(client_identity, name).await?.is_none() {
            return Ok(None);
        }
        let tag = store.tag(client_identity, name);
        let copy = podman::image_id(&tag).await.ok();
        let running = match &copy {
            Some(id) => self.deployer.running_image(id).await,
            None => false,
        };
        if running {
            return Ok(Some(Deletion::InUse));
        }
        podman::podman_in(store.root(), ["rmi", "--force", &tag])
            .await
            .map_err(Error::Podman)?
            .require("podman", |status| status.success())
            .map_err(Error::Podman)?;
        if copy.is_some() {
            let _ = podman::podman(["rmi", &tag]).await;
        }
        store.remove(client_identity, name).await?;
        Ok(Some(Deletion::Deleted))
    }
}
