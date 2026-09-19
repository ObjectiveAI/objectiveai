//! The one call: check, then resize.

use std::path::Path;

use crate::tools::{Error, Finished};

/// Check the filesystem in the image at `image`, then resize it to
/// exactly `bytes / 4096` blocks — growing or shrinking, one call —
/// so it never disagrees with the file's length once the caller has
/// set that. The file is touched by the tools only: growing the file
/// before, or shrinking it after, is the caller's.
///
/// `e2fsck -f -y` exits `0` for a clean filesystem and `1` for one it
/// corrected; either is a filesystem to resize. Any other exit is
/// [`Error::Status`], and so is `resize2fs` exiting other than `0` —
/// which is how a shrink the content fits but the metadata does not
/// is refused, with the filesystem as it was.
pub async fn resize(image: &Path, bytes: u64) -> Result<(), Error> {
    let path = path(image);
    tool("e2fsck", &["-f".to_string(), "-y".to_string(), path.clone()])
        .await?
        .require("e2fsck", |status| matches!(status.code(), Some(0 | 1)))?;
    tool("resize2fs", &[path, (bytes / 4096).to_string()])
        .await?
        .require("resize2fs", |status| status.success())
}

/// The tool by name, on this host's `PATH`, run natively.
#[cfg(target_os = "linux")]
async fn tool(program: &str, args: &[String]) -> Result<Finished, Error> {
    let mut command = tokio::process::Command::new(program);
    command.args(args);
    crate::tools::run(program, command).await
}

/// The tool inside the podman machine, as root there: the provider's
/// own host has no e2fsprogs to speak of, and the machine is where
/// the image is loop-mounted anyway.
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
