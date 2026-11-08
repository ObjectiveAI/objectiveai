//! The manager over the Store.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use futures_util::{future, stream};

use super::{Build, Error, Push, Served, Store, build, push, resolve};
use crate::host::container_deployer::ContainerDeployer;
use crate::host::image_registry::Digest;
use crate::host::registry_client::RegistryClient;
use crate::host::tools::podman;
use crate::host::volume_manager::VolumeManager;
use crate::protocol::context::Context;
use crate::protocol::image_manager;
use crate::protocol::image_source::ImageSource;
use crate::protocol::oci_store;
use crate::protocol::progress::{Progress, Pushing};
use diverge_sdk::provider::endpoints::images::delete::server::response::Deletion;
use diverge_sdk::provider::endpoints::images::list::server::response::Image;
use diverge_sdk::provider::endpoints::images::stat::server::response::Stat;
use diverge_sdk::shared::containers::request::Image as Wanted;
use diverge_sdk::shared::discover::Description;

/// This provider's image manager: the Store, the blob store an image
/// a registry serves is brought through, the volumes a build reads its
/// context from, the deployer a build and a delete reach the cache
/// through, and the registry client a push and a pull fetch with.
///
/// It is the provider's [`OciStore`](oci_store::OciStore) as well as
/// its [`ImageManager`](image_manager::ImageManager): a pull opens
/// from one resolution — see [`resolve`] — so an image is held
/// exactly where the scope serves it and proves it.
#[derive(Debug)]
pub struct ImageManager {
    /// The Store; `None` where the configuration has none.
    store: Option<Arc<Store>>,
    /// The blob store: the Store, or — with none — the deployer's own
    /// with no room, which an image a registry serves is brought
    /// through for a pull.
    blobs: Arc<Store>,
    /// The volumes, for a build's context.
    volumes: Arc<VolumeManager>,
    /// The deployer: its mounts directory, auth file and registry
    /// port for a build, its cache for a delete's in-use answer.
    deployer: Arc<ContainerDeployer>,
    /// The client of the registries the configuration lists.
    registries: Arc<RegistryClient>,
}

impl ImageManager {
    /// A manager over `store` and the blob store `blobs`, with the
    /// volumes, the deployer and the registry client.
    pub fn new(
        store: Option<Arc<Store>>,
        blobs: Arc<Store>,
        volumes: Arc<VolumeManager>,
        deployer: Arc<ContainerDeployer>,
        registries: Arc<RegistryClient>,
    ) -> Self {
        ImageManager {
            store,
            blobs,
            volumes,
            deployer,
            registries,
        }
    }
}

impl image_manager::ImageManager for ImageManager {
    type Error = Error;

    /// The identity's ownership records; none with no Store.
    async fn list(&self, client_identity: &Identity) -> Result<Vec<Image>, Error> {
        match &self.store {
            Some(store) => store.list(client_identity).await,
            None => Ok(Vec::new()),
        }
    }

    /// The identity's record of `digest`, with its Containerfile. A
    /// digest that does not parse is owned by nobody.
    async fn stat(&self, client_identity: &Identity, digest: &str) -> Result<Option<Stat>, Error> {
        let (Some(store), Some(parsed)) = (&self.store, Digest::parse(digest)) else {
            return Ok(None);
        };
        let Some(meta) = store.owned(client_identity, &parsed).await? else {
            return Ok(None);
        };
        Ok(Some(Stat {
            image: Image {
                digest: parsed.to_string(),
                description: meta.description,
                bytes: meta.bytes,
                created: meta.created,
            },
            containerfile: meta.containerfile,
        }))
    }

    type Build = Build;

    /// The build, on a task of its own; with no Store, the one error.
    async fn build(&self, client_identity: &Identity, description: Option<&Description>, containerfile: &str, context: Option<&Context>) -> Self::Build {
        match &self.store {
            Some(store) => build(
                Arc::clone(store),
                Arc::clone(&self.volumes),
                Arc::clone(&self.deployer),
                client_identity,
                description,
                containerfile,
                context,
            ),
            None => Box::pin(stream::once(future::ready(Err::<Progress, Error>(Error::Absent)))),
        }
    }

    type Push = Push;

    /// The push, on a task of its own; with no Store, the one error.
    async fn push(&self, client_identity: &Identity, image: &Wanted, description: Option<&Description>, source: ImageSource) -> Self::Push {
        match &self.store {
            Some(store) => push(Arc::clone(store), Arc::clone(&self.registries), client_identity, image, description, source),
            None => Box::pin(stream::once(future::ready(Err::<Pushing, Error>(Error::Absent)))),
        }
    }

    /// The description set on the identity's record; `None` for a
    /// digest it owns no image by.
    async fn edit(&self, client_identity: &Identity, digest: &str, description: Option<&Description>) -> Result<Option<bool>, Error> {
        let (Some(store), Some(parsed)) = (&self.store, Digest::parse(digest)) else {
            return Ok(None);
        };
        store.describe(client_identity, &parsed, description.cloned()).await
    }

    /// The ownership removed and the Store pruned of what nothing else
    /// reaches; in use where a container runs the image's copy in the
    /// cache. The cache's copy, when one is held under the provider's
    /// own registry's names alone and nobody else holds the digest,
    /// is removed too. `None` for a digest the identity owns no image
    /// by.
    async fn delete(&self, client_identity: &Identity, digest: &str) -> Result<Option<Deletion>, Error> {
        let (Some(store), Some(parsed)) = (&self.store, Digest::parse(digest)) else {
            return Ok(None);
        };
        if store.owned(client_identity, &parsed).await?.is_none() {
            return Ok(None);
        }
        let copy = podman::image_by_digest_any(digest).await.ok().flatten();
        if let Some((id, _)) = &copy {
            if self.deployer.running_image(id).await {
                return Ok(Some(Deletion::InUse));
            }
        }
        if !store.disown(client_identity, &parsed).await? {
            return Ok(None);
        }
        if let Some((id, own)) = copy {
            if !own && !store.owned_by_anyone(&parsed).await? {
                let removed = podman::podman(["rmi", &id]).await.is_ok_and(|finished| finished.status.success());
                if removed {
                    self.deployer.forget_image(&id).await;
                }
            }
        }
        Ok(Some(Deletion::Deleted))
    }
}

impl oci_store::OciStore for ImageManager {
    type Error = Error;

    type Served = Served;

    /// The image as [`resolve`] finds it for the caller, opened for
    /// one scope — see [`Served`]. A resolution that fails to answer
    /// is the error; one that finds nothing, or a registry's manifest
    /// that could not be written to the blob store, serves nothing.
    async fn open(&self, client_identity: &Identity, image: &Wanted) -> Result<Option<Served>, Error> {
        let Some(resolved) = resolve(self.store.as_deref(), &self.registries, client_identity, image).await? else {
            return Ok(None);
        };
        Ok(Served::open(resolved, self.store.as_ref(), &self.blobs, &self.registries, client_identity).await)
    }
}
