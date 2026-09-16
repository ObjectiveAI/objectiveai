//! The machine podman runs in, on the hosts that have one.

use std::path::Path;

use super::command;
use crate::tools::{Error, Finished, run};

/// Run `program` with `args` inside the podman machine, as root
/// there: `podman machine ssh -- sudo <program> <args…>`. The error
/// names `program`, not `podman`, since the program is what the
/// caller asked for.
pub async fn sudo(program: &str, args: &[String]) -> Result<Finished, Error> {
    let mut command = command(["machine", "ssh", "--", "sudo", program]);
    command.args(args);
    run(program, command).await
}

/// A host path as the machine sees it: on macOS, the path itself,
/// since a podman machine mounts a host directory at the same path
/// inside the VM — `$HOME` by default, and any other directory the
/// machine was created with, which a store outside `$HOME` needs
/// for the loop mount as much as for this.
#[cfg(target_os = "macos")]
pub fn path(host: &Path) -> String {
    host.to_string_lossy().into_owned()
}

/// A host path as the machine sees it: on Windows, `C:\a\b` is
/// `/mnt/c/a/b`, where the machine's WSL distribution mounts every
/// drive. A path with no drive letter — a UNC path — is passed as
/// written, and the tool says it cannot find it.
#[cfg(windows)]
pub fn path(host: &Path) -> String {
    let written = host.to_string_lossy();
    let mut chars = written.chars();
    match (chars.next(), chars.next()) {
        (Some(drive), Some(':')) if drive.is_ascii_alphabetic() => {
            let rest: String = chars.collect();
            format!("/mnt/{}{}", drive.to_ascii_lowercase(), rest.replace('\\', "/"))
        }
        _ => written.into_owned(),
    }
}
