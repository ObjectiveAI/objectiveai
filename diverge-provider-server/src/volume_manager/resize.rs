//! Resizing the filesystem inside an image, with the system's
//! e2fsprogs.
//!
//! Nothing in this crate can resize an ext4 filesystem: the
//! formatter cannot, and podman has no such verb. `resize2fs` can,
//! offline, on the image file itself, with no loop device and no
//! privilege — and it refuses a filesystem that was mounted since it
//! was last checked, so `e2fsck` runs first. Both are assumed to be
//! where this host can reach them: on Linux, on the provider's own
//! `PATH`; on macOS and Windows, inside the podman machine, which is
//! Fedora CoreOS and ships them, reached over `podman machine ssh`
//! with the image's path as the machine sees it.

use std::path::Path;
use std::process::{ExitStatus, Stdio};

use tokio::process::Command;

use super::Error;

/// Check the filesystem in the image at `path`, then resize it to
/// exactly `bytes / 4096` blocks — growing or shrinking, one call —
/// so it never disagrees with the file's length once the caller has
/// set that. The file is touched by the tools only: growing the file
/// before, or shrinking it after, is the caller's.
///
/// `e2fsck -f -y` exits `0` for a clean filesystem and `1` for one it
/// corrected; either is a filesystem to resize. Any other exit is
/// [`Error::Tool`], and so is `resize2fs` exiting other than `0` —
/// which is how a shrink the content fits but the metadata does not
/// is refused, with the filesystem as it was.
pub async fn resize(path: &Path, bytes: u64) -> Result<(), Error> {
    let image = machine_path(path);
    let checked = run("e2fsck", &["-f".to_string(), "-y".to_string(), image.clone()]).await?;
    if !matches!(checked.code(), Some(0 | 1)) {
        return Err(checked_error("e2fsck", checked));
    }
    let resized = run("resize2fs", &[image, (bytes / 4096).to_string()]).await?;
    if !resized.success() {
        return Err(checked_error("resize2fs", resized));
    }
    Ok(())
}

/// The status and what the tool wrote, together.
struct Finished {
    status: ExitStatus,
    stderr: String,
}

impl Finished {
    fn code(&self) -> Option<i32> {
        self.status.code()
    }

    fn success(&self) -> bool {
        self.status.success()
    }
}

/// The error for a tool that ran and refused.
fn checked_error(program: &str, finished: Finished) -> Error {
    Error::Tool {
        program: program.to_string(),
        status: finished.status,
        stderr: finished.stderr,
    }
}

/// Run one of the tools with `args`, where this host reaches it, and
/// wait for it. Stdout is discarded — the tools narrate — and stderr
/// is kept for the error.
async fn run(tool: &str, args: &[String]) -> Result<Finished, Error> {
    let output = command(tool, args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|source| Error::Spawn {
            program: tool.to_string(),
            source,
        })?;
    Ok(Finished {
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// The tool by name, on this host's `PATH`.
#[cfg(target_os = "linux")]
fn command(tool: &str, args: &[String]) -> Command {
    let mut command = Command::new(tool);
    command.args(args);
    command
}

/// The tool inside the podman machine, as root there, over the
/// machine's SSH: the provider's own host has no e2fsprogs to speak
/// of, and the machine is where the image is loop-mounted anyway.
#[cfg(not(target_os = "linux"))]
fn command(tool: &str, args: &[String]) -> Command {
    let mut command = Command::new("podman");
    command.args(["machine", "ssh", "--", "sudo", tool]).args(args);
    command
}

/// The image's path as the tool sees it: on Linux, the path itself.
#[cfg(target_os = "linux")]
fn machine_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The image's path as the tool sees it: on macOS, the path itself,
/// since a podman machine mounts a host directory at the same path
/// inside the VM — `$HOME` by default, and any other directory the
/// machine was created with, which a store outside `$HOME` needs
/// for the loop mount as much as for this.
#[cfg(target_os = "macos")]
fn machine_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The image's path as the tool sees it: on Windows, `C:\a\b` is
/// `/mnt/c/a/b`, where the machine's WSL distribution mounts every
/// drive. A path with no drive letter — a UNC path — is passed as
/// written, and the tool says it cannot find it.
#[cfg(windows)]
fn machine_path(path: &Path) -> String {
    let written = path.to_string_lossy();
    let mut chars = written.chars();
    match (chars.next(), chars.next()) {
        (Some(drive), Some(':')) if drive.is_ascii_alphabetic() => {
            let rest: String = chars.collect();
            format!("/mnt/{}{}", drive.to_ascii_lowercase(), rest.replace('\\', "/"))
        }
        _ => written.into_owned(),
    }
}
