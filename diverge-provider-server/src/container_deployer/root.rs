//! Whether the provider is root, on the host where podman is its
//! own.

use super::Error;

/// The provider must be root on Linux: podman is run from its own
/// account, and what a deploy does — a volume image loop-mounted,
/// a store under `storage_path`, `/dev/fuse` handed to a container
/// with the privilege to mount, a container's disk kept by quota —
/// is root's. Read from the kernel's own account of this process,
/// `/proc/self/status`, whose `Uid:` line holds the real, effective,
/// saved and filesystem ids; the effective one is what podman runs
/// as. Anything but `0` there, or a status that cannot be read, is
/// [`Error::Root`].
pub(super) async fn ensure() -> Result<(), Error> {
    let status = tokio::fs::read_to_string("/proc/self/status").await?;
    let effective = status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|ids| ids.split_whitespace().nth(1))
        .and_then(|id| id.parse::<u32>().ok());
    match effective {
        Some(0) => Ok(()),
        _ => Err(Error::Root),
    }
}
