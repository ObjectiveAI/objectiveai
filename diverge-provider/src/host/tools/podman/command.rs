//! A podman invocation.

use std::ffi::OsStr;
use std::path::PathBuf;
use tokio::process::Command;
use tokio::sync::OnceCell;

use super::super::{Finished, run};
use crate::host::tools::Error;

/// Where podman keeps its data, once the configuration has said:
/// what every invocation is told. Set once, before the first
/// invocation, by [`configure`].
static STORAGE: OnceCell<PathBuf> = OnceCell::const_new();

/// The name of the provider's machine, on the hosts with one: the
/// provider's own, never `podman-machine-default`, which is the name
/// podman gives a machine made without one and so the name any other
/// program's machine, or a person's, is likely to carry. Every
/// machine subcommand names it, since podman's "default" machine is
/// whichever one was made first on the host, and may be another
/// program's.
#[cfg(not(target_os = "linux"))]
pub const MACHINE: &str = "diverge-provider";

/// The connection every other subcommand is told to use, on the
/// hosts with a machine: the rootful connection podman registers for
/// [`MACHINE`], named `<machine>-root`, since podman inside the
/// machine runs as root. Podman's default connection is likewise
/// whichever machine was registered first, and is not changed.
#[cfg(not(target_os = "linux"))]
const CONNECTION: &str = "diverge-provider-root";

/// Tell every podman invocation to come where podman's data is:
/// `containers.podman.storage_path`, resolved. Once; a second call
/// changes nothing.
pub fn configure(storage: PathBuf) {
    let _ = STORAGE.set(storage);
}

/// `podman` with `args`, as a command not yet run: the root every
/// podman invocation grows from, so what podman is called and how is
/// decided here once. Podman is named bare and found on `PATH`. The
/// storage [`configure`] gave is on every invocation: on Linux as
/// `--root`, before the subcommand, so the store is that directory;
/// on macOS and Windows as both `XDG_DATA_HOME`, under which podman
/// keeps a machine's disk and SSH key, and `XDG_CONFIG_HOME`, under
/// which it keeps the machine's description and its connection
/// where it honours the variable — on Windows it does not, and the
/// descriptions and connections of every machine on the host stay
/// under `~/.config`, shared with whatever else made one. So on
/// those hosts every invocation also carries `--connection` naming
/// [`MACHINE`]'s rootful connection, before the subcommand, which
/// the machine subcommands ignore and every other subcommand speaks
/// through; and the machine subcommands name the machine. The data
/// home puts the machine's files under the storage path, so the
/// path IS the machine: a different path is a different machine,
/// made fresh, and the one under the old path is left as it was.
/// Everything the provider needs of podman is on the command line.
pub fn command<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("podman");
    if let Some(storage) = STORAGE.get() {
        #[cfg(target_os = "linux")]
        command.arg("--root").arg(storage);
        #[cfg(not(target_os = "linux"))]
        command.env("XDG_DATA_HOME", storage).env("XDG_CONFIG_HOME", storage);
    }
    #[cfg(not(target_os = "linux"))]
    command.arg("--connection").arg(CONNECTION);
    command.args(args);
    command
}

/// `podman` with `args`, run and waited for.
pub async fn podman<I, S>(args: I) -> Result<Finished, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run("podman", command(args)).await
}
