//! The two calls: on, and off.

use std::path::Path;

use crate::tools::{Error, Finished};

/// Make the directory `dir` and mount the filesystem in the image at
/// `image` on it, through a loop device the kernel picks. `dir` is a
/// path as the tool sees it — the host's on Linux, the machine's
/// elsewhere — and is what podman is handed as the bind's source.
pub async fn mount(image: &Path, dir: &str) -> Result<(), Error> {
    tool("mkdir", &["-p".to_string(), dir.to_string()])
        .await?
        .require("mkdir", |status| status.success())?;
    tool("mount", &["-o".to_string(), "loop".to_string(), path(image), dir.to_string()])
        .await?
        .require("mount", |status| status.success())
}

/// Unmount what [`mount`] mounted on `dir` and remove the directory.
/// The loop device goes with the mount.
pub async fn unmount(dir: &str) -> Result<(), Error> {
    tool("umount", &[dir.to_string()])
        .await?
        .require("umount", |status| status.success())?;
    tool("rmdir", &[dir.to_string()])
        .await?
        .require("rmdir", |status| status.success())
}

/// The tool by name, on this host's `PATH`, run natively as the root
/// the provider is.
#[cfg(target_os = "linux")]
async fn tool(program: &str, args: &[String]) -> Result<Finished, Error> {
    let mut command = tokio::process::Command::new(program);
    command.args(args);
    crate::tools::run(program, command).await
}

/// The tool inside the podman machine, as root there, where the bind
/// source has to be.
#[cfg(not(target_os = "linux"))]
async fn tool(program: &str, args: &[String]) -> Result<Finished, Error> {
    crate::tools::podman::sudo(program, args).await
}

/// The image's path as the tool sees it: on Linux, the path itself.
#[cfg(target_os = "linux")]
fn path(image: &Path) -> String {
    image.to_string_lossy().into_owned()
}

/// The image's path as the tool sees it: the machine's view of it.
#[cfg(not(target_os = "linux"))]
fn path(image: &Path) -> String {
    crate::tools::podman::path(image)
}
