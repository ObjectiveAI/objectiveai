//! The mounts, answered.

use bytes::Bytes;
use diverge_sdk::provider::client::{FuseServer, Listed};
use diverge_sdk::shared::containers::fuse::Attrs;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::containers::fuse::stat::Stat;

use super::Answerer;

/// Every ask to the run's [`Mounts`](crate::containers::fuse::Mounts),
/// by the id the mount was named by; each a use of the container.
impl FuseServer for Answerer {
    async fn stat(&self, id: &str, path: &str) -> Result<Option<Stat>, String> {
        self.touch();
        self.mounts.stat(id, path).await
    }

    async fn read(&self, id: &str, path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
        self.touch();
        self.mounts.read(id, path, offset, length).await
    }

    async fn write(&self, id: &str, path: &str, offset: u64, bytes: Bytes) -> Result<(), Refused> {
        self.touch();
        self.mounts.write(id, path, offset, bytes).await
    }

    async fn truncate(&self, id: &str, path: &str, size: u64) -> Result<(), Refused> {
        self.touch();
        self.mounts.truncate(id, path, size).await
    }

    async fn setattr(&self, id: &str, path: &str, attrs: Attrs) -> Result<(), Refused> {
        self.touch();
        self.mounts.setattr(id, path, attrs).await
    }

    async fn list(&self, id: &str, path: &str) -> Result<Option<Vec<Listed>>, String> {
        self.touch();
        self.mounts.list(id, path).await
    }

    async fn remove(&self, id: &str, path: &str) -> Result<(), Refused> {
        self.touch();
        self.mounts.remove(id, path).await
    }

    async fn rename(&self, id: &str, from: &str, to: &str) -> Result<(), Refused> {
        self.touch();
        self.mounts.rename(id, from, to).await
    }

    async fn mkdir(&self, id: &str, path: &str) -> Result<(), Refused> {
        self.touch();
        self.mounts.mkdir(id, path).await
    }
}
