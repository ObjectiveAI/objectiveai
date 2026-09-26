//! The podman machine, made or brought to what the configuration
//! says, on the hosts that have one.

use std::path::{Path, PathBuf};

use super::Error;
use crate::tools::podman;

/// The machine as the configuration wants it, before anything else
/// asks podman: made if there is none, with podman inside it as
/// root; set to `memory` on macOS, where a machine holds its memory
/// from the host and WSL on Windows gives what it gives; seeing every
/// directory of `shares` on macOS, where a machine sees `$HOME` and
/// what it was made with and nothing else — on Windows WSL mounts
/// every drive of the host, and the WSL machine takes no share;
/// running when this returns. Every podman invocation is told to keep
/// the machine under `storage`, so a machine podman lists is one made
/// under this path, and a changed path is a fresh machine.
///
/// A setting is changed with the machine stopped, as podman requires,
/// and the machine started after. A share is not a setting: a machine
/// on macOS that does not see every directory of `shares` — a store
/// or a fixed volume added since it was made — is stopped, removed
/// and made again seeing them, which empties its image cache and
/// drops any image loaded into it by hand, to be loaded again. A
/// machine whose disk is not under `storage` is podman not keeping
/// the machine where the environment put it, and the provider cannot
/// run on a machine it does not control: the refusal names the disk,
/// and a machine the provider itself just made is removed again
/// first.
pub(super) async fn ensure(memory: u64, storage: &Path, shares: &[PathBuf]) -> Result<(), Error> {
    let Some(machine) = podman::machine().await.map_err(Error::Podman)? else {
        return make(memory, storage, shares).await;
    };
    if !machine.image.starts_with(storage) {
        return Err(Error::Machine(machine.image));
    }
    if cfg!(target_os = "macos") && !sees_all(&machine.mounts, shares) {
        if machine.state == "running" {
            machine_command(["stop"]).await?;
        }
        machine_command(["rm", "--force"]).await?;
        return make(memory, storage, shares).await;
    }
    let set_rootful = !machine.rootful;
    let set_memory = cfg!(target_os = "macos") && machine.memory != mib(memory);
    if set_rootful || set_memory {
        if machine.state == "running" {
            machine_command(["stop"]).await?;
        }
        if set_rootful {
            machine_command(["set", "--rootful"]).await?;
        }
        if set_memory {
            machine_command(["set", "--memory", &mib(memory).to_string()]).await?;
        }
        return start().await;
    }
    if machine.state != "running" {
        return start().await;
    }
    Ok(())
}

/// A machine made, its disk checked to be under `storage`, and
/// started.
async fn make(memory: u64, storage: &Path, shares: &[PathBuf]) -> Result<(), Error> {
    init(memory, shares).await?;
    let made = podman::machine()
        .await
        .map_err(Error::Podman)?
        .ok_or_else(|| Error::Machine(PathBuf::new()))?;
    if !made.image.starts_with(storage) {
        let _ = podman::podman(["machine", "rm", "--force"]).await;
        return Err(Error::Machine(made.image));
    }
    start().await
}

/// `podman machine init`: root inside, and on macOS the memory and
/// every share, as `--volume <path>:<path>` — the same path inside
/// as on the host, which is what the machine's view of a host path
/// assumes.
async fn init(memory: u64, shares: &[PathBuf]) -> Result<(), Error> {
    let mib = mib(memory).to_string();
    let volumes: Vec<String> = fewest(shares)
        .into_iter()
        .map(|share| format!("{}:{}", share.display(), share.display()))
        .collect();
    let mut args = vec!["init", "--rootful"];
    if cfg!(target_os = "macos") {
        args.push("--memory");
        args.push(&mib);
        for volume in &volumes {
            args.push("--volume");
            args.push(volume);
        }
    }
    machine_command(args).await
}

/// `podman machine start`.
async fn start() -> Result<(), Error> {
    machine_command(["start"]).await
}

/// `podman machine <args>`, required to succeed.
async fn machine_command<'a>(args: impl IntoIterator<Item = &'a str>) -> Result<(), Error> {
    let args: Vec<&str> = std::iter::once("machine").chain(args).collect();
    podman::podman(args)
        .await
        .map_err(Error::Podman)?
        .require("podman", |status| status.success())
        .map_err(Error::Podman)
}

/// Whether every share lies under some directory the machine mounts.
fn sees_all(mounts: &[PathBuf], shares: &[PathBuf]) -> bool {
    shares.iter().all(|share| mounts.iter().any(|mount| share.starts_with(mount)))
}

/// The shares with none inside another: a directory under one
/// already given is seen through it, and a share given twice is
/// given once.
fn fewest(shares: &[PathBuf]) -> Vec<&Path> {
    let mut fewest: Vec<&Path> = Vec::new();
    for share in shares {
        if fewest.iter().any(|kept| share.starts_with(kept)) {
            continue;
        }
        fewest.retain(|kept| !kept.starts_with(share));
        fewest.push(share);
    }
    fewest
}

/// Bytes as the MiB podman counts a machine's memory in.
fn mib(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}
