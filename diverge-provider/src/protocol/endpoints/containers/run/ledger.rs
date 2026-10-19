//! The mounts a running container holds, for one asked for while it
//! runs to be checked against.

use diverge_sdk::shared::containers::request::Container;

use super::check;

/// Every mount the container holds, by path — the request's volume
/// mounts and FUSE mounts, and every FUSE mount made since on the
/// caller's `fuse_mount` channel — with each FUSE mount's id. A mount
/// asked for while the container runs is claimed here BEFORE the
/// proxy is asked, so that two asked for at once cannot both be
/// made, and released when the proxy does not make it.
///
/// The rules of a claim, in order:
///
/// 1. An id a FUSE mount is held under — the request's, or one made
///    since — is `Taken`.
/// 2. A path that is a held mount's, lies inside one, or holds one —
///    a volume mount's or a FUSE mount's — is `Overlaps`.
/// 3. Else the mount is held from now, under the id, at the path.
#[derive(Debug)]
pub(crate) struct Ledger {
    /// The volume mounts' paths.
    volumes: Vec<Vec<String>>,
    /// The FUSE mounts, held or claimed.
    fuse: Vec<Entry>,
}

/// One FUSE mount the container holds.
#[derive(Debug)]
struct Entry {
    id: String,
    path: Vec<String>,
}

/// Why a claim is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The path is a held mount's, inside one, or holds one.
    Overlaps,
    /// A FUSE mount is held under the id.
    Taken,
}

impl Ledger {
    /// The mounts of the request: every volume mount's path, and
    /// every FUSE mount of either list under its id.
    pub(crate) fn new(request: &Container) -> Ledger {
        Ledger {
            volumes: request.volume_mounts.iter().map(|mount| mount.container_path.clone()).collect(),
            fuse: request
                .fuse_file_mounts
                .iter()
                .chain(&request.fuse_directory_mounts)
                .map(|mount| Entry {
                    id: mount.id.clone(),
                    path: mount.container_path.clone(),
                })
                .collect(),
        }
    }

    /// Hold a FUSE mount under `id` at `path`, or say why not.
    pub(crate) fn claim(&mut self, id: &str, path: &[String]) -> Result<(), Refusal> {
        if self.fuse.iter().any(|entry| entry.id == id) {
            return Err(Refusal::Taken);
        }
        if self.volumes.iter().chain(self.fuse.iter().map(|entry| &entry.path)).any(|held| check::overlaps(held, path)) {
            return Err(Refusal::Overlaps);
        }
        self.fuse.push(Entry {
            id: id.to_string(),
            path: path.to_vec(),
        });
        Ok(())
    }

    /// Let go of the claim under `id`: the proxy did not make it.
    pub(crate) fn release(&mut self, id: &str) {
        self.fuse.retain(|entry| entry.id != id);
    }
}
