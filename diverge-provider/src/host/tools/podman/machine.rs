//! The machine podman runs in, on the hosts that have one.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{MACHINE, command};
use crate::host::tools::{Error, Finished, run};

/// Run `program` with `args` inside the podman machine, as root
/// there: `podman machine ssh <machine> -- sudo <program> <args…>`.
/// The error names `program`, not `podman`, since the program is
/// what the caller asked for.
pub async fn sudo(program: &str, args: &[String]) -> Result<Finished, Error> {
    let mut command = command(["machine", "ssh", MACHINE, "--", "sudo", program]);
    command.args(args);
    run(program, command).await
}

/// `podman` with `args`, run INSIDE the podman machine, as root
/// there: `podman machine ssh <machine> -- sudo podman <args…>`. For
/// the one subcommand that reads the filesystem it is run on — a
/// build, whose Containerfile and context are paths — where the
/// client on this host would read this host's filesystem, and the
/// paths the provider holds are the machine's: a stored volume is
/// mounted in the machine and nowhere else. Root's podman in the
/// machine keeps the store the provider's connection speaks to, so
/// what it builds is there for every invocation after.
///
/// The arguments cross a shell on the way, so each is quoted whole:
/// a path with a space in it stays one argument.
pub fn inside<I, S>(args: I) -> tokio::process::Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut command = command(["machine", "ssh", MACHINE, "--", "sudo", "podman"]);
    for arg in args {
        command.arg(format!("'{}'", arg.as_ref().replace('\'', r"'\''")));
    }
    command
}

/// A host path as the machine sees it: on macOS, the path itself,
/// since a podman machine mounts a host directory at the same path
/// inside the VM — every directory the provider made the machine
/// seeing: each store, each fixed volume, the provider's directory
/// and the executable's.
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

/// A machine's description, `<ConfigDir>/<machine>.json`: the file
/// podman wrote when the machine was made, and starts it from. Its
/// mounts are all that is read, and it may list none.
#[derive(Deserialize)]
struct DescriptionRow {
    #[serde(rename = "Mounts", default)]
    mounts: Option<Vec<MountRow>>,
}

/// One host directory a machine mounts.
#[derive(Deserialize)]
struct MountRow {
    #[serde(rename = "Source")]
    source: PathBuf,
}

/// The host directories the provider's machine was made seeing, as
/// the description podman keeps of it lists them: the `Mounts` of
/// `<config>/<machine>.json`, `config` being the `ConfigDir` of the
/// machine's inspection. `podman machine init --volume` wrote them
/// there, and `podman machine inspect` does not repeat them.
///
/// A description that lists none is a machine seeing nothing. One
/// that is not there, or does not read as one, is podman's answer
/// unreadable, as an inspection that does not parse is: the machine
/// is not taken for one seeing nothing on that, which would have it
/// made again and its image cache emptied at every start.
pub async fn described(config: &Path) -> Result<Vec<PathBuf>, Error> {
    let description = tokio::fs::read(config.join(format!("{MACHINE}.json"))).await.map_err(|_| Error::Output {
        program: "podman".to_string(),
    })?;
    mounts(&description)
}

/// The mounts the bytes of a description list.
fn mounts(description: &[u8]) -> Result<Vec<PathBuf>, Error> {
    let row: DescriptionRow = serde_json::from_slice(description).map_err(|_| Error::Output {
        program: "podman".to_string(),
    })?;
    Ok(row.mounts.unwrap_or_default().into_iter().map(|mount| mount.source).collect())
}
