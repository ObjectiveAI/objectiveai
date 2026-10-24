//! The reporter.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use diverge_sdk::provider::endpoints::capacity::server::response::Frame;
use futures_util::future;

use super::Error;
use crate::host::container_deployer::ContainerDeployer;
use crate::host::image_manager::Store;
use crate::host::volume_manager::VolumeManager;
use crate::protocol::capacity_reporter;
use crate::protocol::volume_manager;

/// The host's capacity reporter: the volumes' largest store room,
/// the client image store's room, and the deployer's two caps, each
/// as of now.
#[derive(Debug)]
pub struct CapacityReporter {
    /// The volumes, whose stores hold the room for a new one.
    volumes: Arc<VolumeManager>,
    /// The client image store; `None` where the configuration has
    /// none, and then its room is `0`.
    store: Option<Arc<Store>>,
    /// The deployer, whose caps hold the memory and the overlay room.
    deployer: Arc<ContainerDeployer>,
}

impl CapacityReporter {
    /// A reporter over the volumes, the client image store if there
    /// is one, and the deployer.
    pub fn new(volumes: Arc<VolumeManager>, store: Option<Arc<Store>>, deployer: Arc<ContainerDeployer>) -> Self {
        CapacityReporter { volumes, store, deployer }
    }
}

impl capacity_reporter::CapacityReporter for CapacityReporter {
    type Error = Error;

    /// The four numbers, the two stores measured beside each other:
    /// `volumes_disk` is what `volumes::create_capacity` answers, the
    /// most room any one store has; `images_disk` is the store's
    /// capacity less what its pools hold, or `0` with no store; the
    /// container numbers are what remains of the two caps. The
    /// identity narrows nothing: this host allots every caller the
    /// whole of the room.
    async fn capacity(&self, client_identity: &Identity) -> Result<Frame, Error> {
        let (volumes_disk, images_disk) = future::join(
            volume_manager::VolumeManager::create_capacity(&*self.volumes, client_identity),
            async {
                match &self.store {
                    Some(store) => store.room().await,
                    None => Ok(0),
                }
            },
        )
        .await;
        Ok(Frame {
            volumes_disk: volumes_disk.map_err(Error::Volumes)?,
            images_disk: images_disk.map_err(Error::Images)?,
            container_memory: self.deployer.memory_room(),
            container_disk: self.deployer.disk_room(),
        })
    }
}
